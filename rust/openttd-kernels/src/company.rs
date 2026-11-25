/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
//! The company finance and lifecycle bodies of `company_cmd.cpp`, the company economy of
//! `economy.cpp` (value, rating, ownership transfer, bankruptcy, statistics, prices,
//! inflation, interest and takeover) and the four money commands of `misc_cmd.cpp`, with the
//! finance members of `CompanyProperties` and the globals `_economy`, `_price`, `_score_part`,
//! `_price_base_multiplier` and `_cur_company_tick_index`.
//!
//! `Company` stays a native record: a company is held as `*mut OpenTTDCompany`, resolved once
//! where the original takes its `Company *`, and its finance members as the Rust record
//! [`Finances`] that C++ views in place. Every company, finance, town, vehicle, group or
//! station pointer is a live record serialized by the game-state mutex for the synchronous
//! call; each field access is one field-sized load or store at its observation point and ends
//! before the next call that can reenter or delete (company deletion, ownership changes, pool
//! deletion, nested commands, `AI::Stop`). `AI::StartNew` and the `CCA_NEW_AI` `Post` can raise
//! `Script_FatalError` in ordinary play, so `do_startup_new_company`, `cmd_company_ctrl` and
//! `on_tick_companies` return to a caller-owned C++ frame before them.
use crate::cargo_type::{CargoType, NUM_CARGO};
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_base::COMPANY_MAX_LOAN_DEFAULT;
/// Opaque native `Company`, valid while its pool shell lives.
pub use crate::company_type::OpenTTDCompany;
use crate::company_type::{
    CompanyAllowListCtrlAction, CompanyCtrlAction, CompanyID, CompanyMask, CompanyRemoveReason,
    LoanCommand, MAX_COMPANIES, MAX_HISTORY_QUARTERS, OWNER_END, Owner,
};
use crate::economy_type::{
    ExpensesType, INITIAL_LOAN, LOAN_INTERVAL, MAX_INFLATION, MAX_LOAN_LIMIT, MAX_PRICE_MODIFIER,
    MIN_PRICE_MODIFIER, Price, PriceCategory, SCORE_MAX, ScoreID, ScoreInfo,
};
use crate::network_type::{ClientID, DestType, NetworkAction};
use crate::news_func::{AddNewsItemArgs, add_company_news_item, add_news_item};
use crate::news_type::{CompanyNewsInformation, NewsFlags, NewsStyle, NewsType};
use crate::openttd::GameMode;
use crate::raw_field::{read_field, write_field};
use crate::script_event_types::ScriptEvent;
use crate::services::{ByteSpan, CommandCost, CommandCostServices, Cost};
use crate::services_assert::{assert_with, not_reached};
use crate::sound_type::SoundFx;
use crate::strings_func::{get_encoded_string, get_string};
use crate::strings_type::StringID;
use crate::strings_type::StringParameterData::{self, Integer};
use crate::table::strings::{
    STR_COMPANY_NAME, STR_ERROR_CURRENCY_REQUIRED, STR_ERROR_INSUFFICIENT_FUNDS,
    STR_ERROR_LOAN_ALREADY_REPAID, STR_ERROR_MAXIMUM_PERMITTED_LOAN, STR_ERROR_OWNED_BY,
    STR_ERROR_TOO_MANY_VEHICLES_IN_GAME, STR_NEWS_BEGIN_OF_RECESSION,
    STR_NEWS_COMPANY_BANKRUPT_DESCRIPTION, STR_NEWS_COMPANY_BANKRUPT_TITLE,
    STR_NEWS_COMPANY_IN_TROUBLE_DESCRIPTION, STR_NEWS_COMPANY_IN_TROUBLE_TITLE,
    STR_NEWS_COMPANY_MERGER_DESCRIPTION, STR_NEWS_COMPANY_MERGER_TITLE, STR_NEWS_END_OF_RECESSION,
    STR_NEWS_MERGER_TAKEOVER_TITLE,
};
use crate::tile_type::{TILE_HEIGHT, TILE_SIZE, TileIndex, TileType};
use crate::timer_game_common::{ORIGINAL_BASE_YEAR, ORIGINAL_MAX_YEAR};
use crate::timer_game_tick::{DAY_TICKS, TICKS_PER_SECOND};
use crate::town_type::RATING_INITIAL;
use crate::track_type::TrackBits;
use crate::vehicle::VehicleRef;
use crate::vehicle_func::{VEHICLE_PROFIT_MIN_AGE, is_company_buildable_vehicle_type};
use crate::vehicle_type::VehicleType;
use crate::window_func::{
    close_window_by_id, invalidate_window_classes_data, invalidate_window_data,
    set_window_classes_dirty, set_window_dirty,
};
use crate::window_type::{WindowClass, WindowNumber, WindowNumberEnum};
use std::ffi::c_void;

/// `CompanyEconomyEntry`: statistics about the economy of one quarter.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CompanyEconomyEntry {
    /// The amount of income.
    pub income: i64,
    /// The amount of expenses.
    pub expenses: i64,
    /// The amount of delivered cargo.
    pub delivered_cargo: [u32; NUM_CARGO as usize],
    /// Company score (scale 0-1000).
    pub performance_history: i32,
    /// The value of the company.
    pub company_value: i64,
}
/// The finance members of `CompanyProperties`, stored in Rust; C++ starts its field objects in
/// this storage.
#[repr(C)]
pub struct Finances {
    /// Money owned by the company.
    pub money: i64,
    /// Fraction of money of the company, too small to represent in `money`.
    pub money_fraction: u8,
    /// Amount of money borrowed from the bank.
    pub current_loan: i64,
    /// Max allowed amount of the loan or `COMPANY_MAX_LOAN_DEFAULT`.
    pub max_loan: i64,
    /// Number of quarters that the company is not allowed to get new exclusive engine previews
    /// (see `companies_gen_statistics`).
    pub block_preview: u8,
    /// Number of months this company has not had a client in multiplayer.
    pub months_empty: u8,
    /// Number of months that the company is unable to pay its debts.
    pub months_of_bankruptcy: u8,
    /// Which companies were asked about buying it?
    pub bankrupt_asked: CompanyMask,
    /// If bigger than `0`, amount of time to wait for an answer on an offer to buy this company.
    pub bankrupt_timeout: i16,
    pub bankrupt_value: i64,
    /// Amount of tileheights we can (still) terraform (times 65536).
    pub terraform_limit: u32,
    /// Amount of tiles we can (still) clear (times 65536).
    pub clear_limit: u32,
    /// Amount of trees we can (still) plant (times 65536).
    pub tree_limit: u32,
    /// Amount of tiles we can (still) build objects on (times 65536). Also applies to buying land.
    pub build_object_limit: u32,
    /// Expenses of the company for the last three years.
    pub yearly_expenses: [[i64; ExpensesType::EXPENSES_END.0 as usize]; 3],
    /// Economic data of the company of this quarter.
    pub cur_economy: CompanyEconomyEntry,
    /// Economic data of the company of the last `MAX_HISTORY_QUARTERS` quarters.
    pub old_economy: [CompanyEconomyEntry; MAX_HISTORY_QUARTERS],
    /// Number of valid statistical entries in `old_economy`.
    pub num_valid_stat_ent: u8,
}
/// `Economy`: data of the economy.
#[repr(C)]
pub struct Economy {
    /// `_economy.max_loan`: NOSAVE: Maximum possible loan.
    pub max_loan: i64,
    /// Economy fluctuation status.
    pub fluct: i16,
    /// Interest.
    pub interest_rate: u8,
    /// Inflation amount.
    pub infl_amount: u8,
    /// Inflation rate for payment rates.
    pub infl_amount_pr: u8,
    /// Cumulated inflation of prices since game start; 16 bit fractional part.
    pub inflation_prices: u64,
    /// Cumulated inflation of cargo payment since game start; 16 bit fractional part.
    pub inflation_payment: u64,
    /// Old: Unrounded max loan.
    pub old_max_loan_unround: i64,
    /// Old: Fraction of the unrounded max loan.
    pub old_max_loan_unround_fract: u16,
}

/// `delivered_cargo[cargo] += amount` of the current quarter (`DeliverGoods`).
///
/// # Safety
/// `owner` is a live serialized company finance record and `cargo` a valid cargo slot.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) unsafe fn record_delivery(owner: *mut Finances, cargo: CargoType, amount: u32) {
    let delivered = read_field!(owner, cur_economy.delivered_cargo[usize::from(cargo.0)]);
    write_field!(
        owner,
        cur_economy.delivered_cargo[usize::from(cargo.0)],
        delivered.wrapping_add(amount)
    );
}

/// The finance services the money paths of other owners share.
#[repr(C)]
pub struct FinanceServices {
    /// `_current_company`.
    pub current_company: extern "C" fn() -> u8,
    /// `_current_company = company`.
    pub set_current_company: extern "C" fn(u8),
    /// `_networking`.
    pub networking: extern "C" fn() -> bool,
    /// `&Company::GetIfValid(id)->Finances()`: null for an invalid ID.
    pub company_finances: extern "C" fn(u8) -> *mut Finances,
    /// `&Company::Get(id)->Finances()`: the pool's checked `Get`, asserting
    /// `id < first_unused`.
    pub company_get_finances: extern "C" fn(u8) -> *mut Finances,
    /// `InvalidateCompanyWindows(Company::Get(id))`, which reads only the company's index.
    pub invalidate_company_windows: extern "C" fn(u8),
}

/// The tables and the native callees the company bodies use. A slot that takes a company
/// takes the live `Company` its caller resolved.
#[repr(C)]
pub struct Services {
    /// The command result table.
    pub cost: &'static CommandCostServices,
    /// The finance table.
    pub finance: &'static FinanceServices,
    /// The company identity table: the company pool and `Company` members.
    pub identity: &'static crate::company_identity::Services,
    /// The shared table.
    pub shared: &'static crate::services::Services,
    /// The engine table, resolved at the call.
    pub engines: extern "C" fn() -> *const crate::engine::Services,
    /// The objectives table, resolved at the call.
    pub objectives: extern "C" fn() -> &'static crate::objectives::Services,
    /// The settings table, resolved at the call.
    pub settings: extern "C" fn() -> &'static crate::settings_table::Services,
    /// The station lifecycle table, resolved at the call.
    pub station_lifecycle: extern "C" fn() -> *const crate::station_lifecycle::Services,
    /// The subsidy table, resolved at the call.
    pub subsidies: extern "C" fn() -> *const crate::subsidy::Services,
    /// `AI::CanStartNew()`.
    pub ai_can_start_new: extern "C" fn() -> bool,
    /// `AI::Stop(company)`.
    pub ai_stop: extern "C" fn(CompanyID),
    /// `new Company(STR_SV_UNNAMED, is_ai)`, or `new (company) Company(...)` for a valid
    /// `company`.
    pub allocate_company: extern "C" fn(CompanyID, bool) -> *mut OpenTTDCompany,
    /// `c->allow_list.Add(public_key)`.
    pub allow_list_add: extern "C" fn(*mut OpenTTDCompany, ByteSpan) -> bool,
    /// `c->allow_list.Remove(public_key)`.
    pub allow_list_remove: extern "C" fn(*mut OpenTTDCompany, ByteSpan) -> bool,
    /// `Company::CanAllocateItem()`.
    pub can_allocate_company: extern "C" fn() -> bool,
    /// `CanalMaintenanceCost(num)`.
    pub canal_maintenance_cost: extern "C" fn(u32) -> i64,
    /// `cs->initial_payment` of a cargo spec.
    pub cargo_initial_payment: extern "C" fn(*const std::ffi::c_void) -> i64,
    /// `cs->current_payment = payment` of a cargo spec.
    pub cargo_set_current_payment: extern "C" fn(*const std::ffi::c_void, i64),
    /// `ChangeWindowOwner(old_owner, new_owner)`.
    pub change_window_owner: extern "C" fn(Owner, Owner),
    /// `CloseCompanyWindows(company)`.
    pub close_company_windows: extern "C" fn(CompanyID),
    /// `CloseConstructionWindows()`.
    pub close_construction_windows: extern "C" fn(),
    /// `CompanyAdminRemove(company_id, reason)`.
    pub company_admin_remove: extern "C" fn(CompanyID, CompanyRemoveReason),
    /// `Company::GetNumItems()`.
    pub company_count: extern "C" fn() -> usize,
    /// `delete c`.
    pub delete_company: extern "C" fn(*mut OpenTTDCompany),
    /// `delete g`.
    pub delete_goal: extern "C" fn(*mut crate::goal_type::Goal),
    /// `delete sp`.
    pub delete_story_page: extern "C" fn(*mut crate::story_type::StoryPage),
    /// `GetDrawStringCompanyColour(company)`.
    pub draw_string_company_colour: extern "C" fn(CompanyID) -> crate::gfx_type::TextColour,
    /// `&c->group_all[type]`, the company's statistics of all its vehicles of a type.
    pub group_all: extern "C" fn(*mut OpenTTDCompany, VehicleType) -> *mut (),
    /// `c->location_of_HQ`.
    pub location_of_hq: extern "C" fn(*mut OpenTTDCompany) -> TileIndex,
    /// `NetworkAdminCompanyNew(c)`.
    pub network_admin_company_new: extern "C" fn(*mut OpenTTDCompany),
    /// `NetworkClientsToSpectators(company)`.
    pub network_clients_to_spectators: extern "C" fn(CompanyID),
    /// `_network_server`.
    pub network_server: extern "C" fn() -> bool,
    /// `_new_competitor_timeout.HasFired()`.
    pub new_competitor_timeout_has_fired: extern "C" fn() -> bool,
    /// `_new_competitor_timeout.Reset({TimerGameTick::Priority::COMPETITOR_TIMEOUT, ticks})`.
    pub new_competitor_timeout_reset: extern "C" fn(u32),
    /// `Goal::Iterate(from)`'s next goal.
    pub next_goal: extern "C" fn(u32) -> PoolCursor,
    /// `Sign::Iterate(from)`'s next sign.
    pub next_sign: extern "C" fn(u32) -> PoolCursor,
    /// `StoryPage::Iterate(from)`'s next page.
    pub next_story_page: extern "C" fn(u32) -> PoolCursor,
    /// `Waypoint::Iterate(from)`'s next waypoint.
    pub next_waypoint: extern "C" fn(u32) -> PoolCursor,
    /// `ParseCompanyManagerFaceCode(_company_manager_face)` stored in `c->face`; whether it
    /// parsed.
    pub parse_company_manager_face: extern "C" fn(*mut OpenTTDCompany) -> bool,
    /// `_price_base_specs[price].category`.
    pub price_base_category: extern "C" fn(Price) -> PriceCategory,
    /// `_price_base_specs[price].start_price`.
    pub price_base_start_price: extern "C" fn(Price) -> i64,
    /// `RailMaintenanceCost(railtype, num, total_num)`.
    pub rail_maintenance_cost: extern "C" fn(crate::rail_type::RailType, u32, u32) -> i64,
    /// `RoadMaintenanceCost(roadtype, num, total_num)`.
    pub road_maintenance_cost: extern "C" fn(crate::road_type::RoadType, u32, u32) -> i64,
    /// `ScriptObject::GetRandomizer(OWNER_NONE).Next(max)`.
    pub script_random_next: extern "C" fn(u32) -> u32,
    /// `Command<CMD_SET_COMPANY_MANAGER_FACE>::SendNet(STR_NULL, c->index, ...)` of the parsed
    /// `_company_manager_face`, when it is set and parses.
    pub send_company_manager_face: extern "C" fn(*mut OpenTTDCompany),
    /// `c->avail_railtypes = types`.
    pub set_avail_railtypes: extern "C" fn(*mut OpenTTDCompany, crate::rail_type::RailTypes),
    /// `c->avail_roadtypes = types`.
    pub set_avail_roadtypes: extern "C" fn(*mut OpenTTDCompany, crate::road_type::RoadTypes),
    /// `SetDefaultCompanySettings(company)`.
    pub set_default_company_settings: extern "C" fn(CompanyID),
    /// `c->inaugurated_year = year`.
    pub set_inaugurated_year:
        extern "C" fn(*mut OpenTTDCompany, crate::timer_game_economy::EconomyYear),
    /// `c->inaugurated_year_calendar = year`.
    pub set_inaugurated_year_calendar:
        extern "C" fn(*mut OpenTTDCompany, crate::timer_game_calendar::CalendarYear),
    /// `v->unitnumber = unit`.
    pub set_unitnumber: extern "C" fn(VehicleRef, u16),
    /// `ShowBuyCompanyDialog(company, hostile_takeover)`.
    pub show_buy_company_dialog: extern "C" fn(CompanyID, bool),
    /// `ShowCompanyFinances(company)`.
    pub show_company_finances: extern "C" fn(CompanyID),
    /// `&si->owner`.
    pub sign_owner: extern "C" fn(*mut crate::signs_type::Sign) -> *mut Owner,
    /// `SignalMaintenanceCost(num)`.
    pub signal_maintenance_cost: extern "C" fn(u32) -> i64,
    /// `SndPlayFx(sound)`.
    pub snd_play_fx: extern "C" fn(SoundFx),
    /// `StationMaintenanceCost(num)`.
    pub station_maintenance_cost: extern "C" fn(u32) -> i64,
    /// `SyncCompanySettings()`.
    pub sync_company_settings: extern "C" fn(),
    /// `c->freegroups.UseID(c->freegroups.NextID())`.
    pub use_free_group: extern "C" fn(*mut OpenTTDCompany) -> u16,
    /// `c->freeunits[type].UseID(c->freeunits[type].NextID())`.
    pub use_free_unit: extern "C" fn(*mut OpenTTDCompany, VehicleType) -> u16,
}

/// A live company as the original's `Company *`: the record and its finance members, with its
/// index, resolved once where the original takes it.
#[derive(Clone, Copy)]
pub(crate) struct CompanyRef {
    /// The native record.
    pub(crate) company: *mut OpenTTDCompany,
    /// `&c->Finances()`.
    pub(crate) finances: *mut Finances,
    /// `c->index`.
    pub(crate) index: CompanyID,
}

impl CompanyRef {
    /// The company record `c` a lookup returned.
    pub(crate) fn new(
        identity: &crate::company_identity::Services,
        c: *mut OpenTTDCompany,
    ) -> Self {
        Self {
            company: c,
            finances: (identity.finances)(c),
            index: (identity.index)(c),
        }
    }
    /// `Company::Get(index)`.
    pub(crate) fn get(identity: &crate::company_identity::Services, index: CompanyID) -> Self {
        Self::new(identity, (identity.company)(index))
    }
    /// `Company::GetIfValid(index)`.
    pub(crate) fn get_if_valid(
        identity: &crate::company_identity::Services,
        index: CompanyID,
    ) -> Option<Self> {
        let c = (identity.company_if_valid)(index);
        (!c.is_null()).then(|| Self::new(identity, c))
    }
}

/// `for (Company *c : Company::Iterate())`. The next company is looked up after `body`
/// returns, from the index after the visited one, as the pool's iterator steps, so a company
/// `body` deletes is not visited again.
fn companies(s: &Services, mut body: impl FnMut(CompanyRef)) {
    let mut from = 0_u32;
    loop {
        let c = (s.identity.next_company)(from);
        if c.is_null() {
            return;
        }
        let c = CompanyRef::new(s.identity, c);
        from = u32::from(c.index.0) + 1;
        body(c);
    }
}

/// `_economy`.
#[allow(unsafe_code)] // The zeroed initial value of the plain-integer record.
static mut ECONOMY: Economy = unsafe { std::mem::zeroed() };
/// `_price`.
static mut PRICE: [i64; Price::PR_END.0 as usize] = [0; Price::PR_END.0 as usize];
/// `_score_part`.
static mut SCORE_PART: [[i64; ScoreID::SCORE_END.0 as usize]; MAX_COMPANIES as usize] =
    [[0; ScoreID::SCORE_END.0 as usize]; MAX_COMPANIES as usize];
/// `_price_base_multiplier`.
static mut PRICE_BASE_MULTIPLIER: [i8; Price::PR_END.0 as usize] = [0; Price::PR_END.0 as usize];
/// `_cur_company_tick_index`.
static mut CUR_COMPANY_TICK_INDEX: u32 = 0;

/// `_economy`, for field-sized accesses.
fn economy() -> *mut Economy {
    &raw mut ECONOMY
}

/// `_economy.fluct`.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn economy_fluct() -> i16 {
    read_field!(economy(), fluct)
}

/// `_price[index]`.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn price(index: Price) -> i64 {
    // SAFETY: Process-lifetime storage the game-state mutex serializes; `index` is a `Price`
    // below `PR_END`, as the original's callers pass.
    unsafe {
        (&raw const PRICE)
            .cast::<i64>()
            .add(usize::from(index.0))
            .read()
    }
}

/// Raw adapter of [`price`] for the files no pass has finished.
#[allow(
    clippy::cast_possible_truncation,
    reason = "`_price[index]`: the unfinished callers index by a `Price`, a `uint8_t` enum, widened to `size_t`"
)]
pub(crate) fn price_raw(index: usize) -> i64 {
    price(Price(index as u8))
}

/// `_price[index] = value`.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
fn set_price(index: Price, value: i64) {
    // SAFETY: As `price`.
    unsafe {
        (&raw mut PRICE)
            .cast::<i64>()
            .add(usize::from(index.0))
            .write(value);
    }
}

/// `_score_part[owner][id] = value`.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
fn set_score_part(owner: CompanyID, id: ScoreID, value: i64) {
    // SAFETY: Process-lifetime storage the game-state mutex serializes; `owner` is a company.
    unsafe {
        (&raw mut SCORE_PART)
            .cast::<[i64; ScoreID::SCORE_END.0 as usize]>()
            .add(usize::from(owner.0))
            .cast::<i64>()
            .add(usize::from(id.0))
            .write(value);
    }
}

/// `_score_part[owner][id]`.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
fn score_part(owner: CompanyID, id: ScoreID) -> i64 {
    // SAFETY: As `set_score_part`.
    unsafe {
        (&raw const SCORE_PART)
            .cast::<[i64; ScoreID::SCORE_END.0 as usize]>()
            .add(usize::from(owner.0))
            .cast::<i64>()
            .add(usize::from(id.0))
            .read()
    }
}

/// Score info, values used for computing the detailed performance rating (`_score_info`).
const SCORE_INFO: [ScoreInfo; ScoreID::SCORE_END.0 as usize] = [
    ScoreInfo {
        needed: 120,
        score: 100,
    }, // SCORE_VEHICLES
    ScoreInfo {
        needed: 80,
        score: 100,
    }, // SCORE_STATIONS
    ScoreInfo {
        needed: 10000,
        score: 100,
    }, // SCORE_MIN_PROFIT
    ScoreInfo {
        needed: 50000,
        score: 50,
    }, // SCORE_MIN_INCOME
    ScoreInfo {
        needed: 100_000,
        score: 100,
    }, // SCORE_MAX_INCOME
    ScoreInfo {
        needed: 40000,
        score: 400,
    }, // SCORE_DELIVERED
    ScoreInfo {
        needed: 8,
        score: 50,
    }, // SCORE_CARGO
    ScoreInfo {
        needed: 10_000_000,
        score: 50,
    }, // SCORE_MONEY
    ScoreInfo {
        needed: 250_000,
        score: 50,
    }, // SCORE_LOAN
    ScoreInfo {
        needed: 0,
        score: 0,
    }, // SCORE_TOTAL
];

/// Process-lifetime finance storage of a new company, zeroed; C++ starts its field objects in
/// it at once.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_state_create() -> *mut Finances {
    Box::into_raw(Box::new(unsafe { std::mem::zeroed() }))
}

/// Release the storage after its C++ field objects and every access end.
///
/// # Safety
/// `p` is a live result of `openttd_rust_company_state_create`, owned by the caller and
/// destroyed once.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub unsafe extern "C" fn openttd_rust_company_state_destroy(p: *mut Finances) {
    unsafe { drop(Box::from_raw(p)) };
}

/// `_economy`, on which C++ builds its view once.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_economy_state() -> *mut Economy {
    economy()
}

/// `_price`, on which C++ builds its view once.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_economy_prices() -> *mut i64 {
    (&raw mut PRICE).cast()
}

/// `_score_part`, on which C++ builds its view once.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_scores() -> *mut i64 {
    (&raw mut SCORE_PART).cast()
}

/// `_cur_company_tick_index`, for the savegame and `InitializeCompanies`.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_tick() -> *mut u32 {
    &raw mut CUR_COMPANY_TICK_INDEX
}

/// `Company::Company`'s construction limits: each budget starts at its frame burst.
///
/// # Safety
/// `p` is a live finance record whose C++ view has started, owned by the company being
/// constructed.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub unsafe extern "C" fn openttd_rust_company_initialize(
    p: *mut Finances,
    terraform: u32,
    clear: u32,
    tree: u32,
    build_object: u32,
) {
    write_field!(p, terraform_limit, terraform.wrapping_shl(16));
    write_field!(p, clear_limit, clear.wrapping_shl(16));
    write_field!(p, tree_limit, tree.wrapping_shl(16));
    write_field!(p, build_object_limit, build_object.wrapping_shl(16));
}

/// `OverflowSafeInt64::operator-`: the negation, saturating at the maximum.
pub(crate) fn neg(a: i64) -> i64 {
    if a == i64::MIN { i64::MAX } else { -a }
}

/// `Company::GetMaxLoan`: calculate the max allowed loan for this company.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
fn get_max_loan(c: *const Finances) -> i64 {
    if read_field!(c, max_loan) == COMPANY_MAX_LOAN_DEFAULT {
        return read_field!(economy(), max_loan);
    }
    read_field!(c, max_loan)
}

/// `Company::GetMaxLoan`, for the native member.
///
/// # Safety
/// `p` is a live finance record.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub unsafe extern "C" fn openttd_rust_company_max_loan(p: *const Finances) -> i64 {
    get_max_loan(p)
}

/// `SetLocalCompany`: sets the local company and updates the settings that are set on a
/// per-company basis to reflect the core's state in the GUI. `new_company` is a valid
/// company, `COMPANY_SPECTATOR` or `OWNER_NONE`.
pub(crate) fn set_local_company(s: &Services, new_company: CompanyID) {
    // company could also be COMPANY_SPECTATOR or OWNER_NONE
    assert_with(|| {
        (s.shared.company_valid)(new_company.0)
            || new_company == CompanyID::SPECTATOR
            || new_company == CompanyID::NONE
    });

    // If actually changing to another company, several windows need closing
    let switching_company = (s.shared.local_company)() != new_company.0;

    // Delete the chat window, if you were team chatting.
    if switching_company {
        invalidate_window_data(
            WindowClass::WC_SEND_NETWORK_MSG,
            WindowNumber::from(DestType::DESTTYPE_TEAM),
            i32::from((s.shared.local_company)()),
            false,
        );
    }

    assert_with(|| {
        crate::command::is_local_company_of(s.shared.local_company, s.shared.current_company)
    });

    // `_current_company = _local_company = new_company`.
    (s.shared.set_local_company_id)(new_company.0);
    (s.finance.set_current_company)(new_company.0);

    if switching_company {
        invalidate_window_classes_data(WindowClass::WC_COMPANY, 0, false);
        // Delete any construction windows...
        (s.close_construction_windows)();
    }

    // ... and redraw the whole screen.
    (s.shared.screen_dirty)();
    invalidate_window_classes_data(WindowClass::WC_SIGN_LIST, -1, false);
    invalidate_window_classes_data(WindowClass::WC_GOALS_LIST, 0, false);
    invalidate_window_classes_data(WindowClass::WC_COMPANY_COLOUR, -1, false);
    (s.identity.reset_colourmap)();
}

/// `SetLocalCompany`, for the native function.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_set_local(s: &Services, new_company: CompanyID) {
    set_local_company(s, new_company);
}

/// `GetAvailableMoney`: get the amount of money that a company has available, or `INT64_MAX`
/// if there is no such valid company.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn get_available_money(
    shared: &crate::services::Services,
    f: &FinanceServices,
    company: CompanyID,
) -> i64 {
    // SAFETY: The catalog's address of a live setting.
    if unsafe { shared.settings().infinite_money.read() } {
        return i64::MAX;
    }
    // `Company::IsValidID(company)` fails where `company_finances` is null.
    let c = (f.company_finances)(company.0);
    if c.is_null() {
        return i64::MAX;
    }
    read_field!(c, money)
}

/// `GetAvailableMoney`, for the native function.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_available_money(s: &Services, company: CompanyID) -> i64 {
    get_available_money(s.shared, s.finance, company)
}

/// Expenses types whose cost is income: `EXPENSES_TRAIN_REVENUE` to `EXPENSES_SHIP_REVENUE`.
const REVENUE_EXPENSES: u32 = 1 << ExpensesType::EXPENSES_TRAIN_REVENUE.0
    | 1 << ExpensesType::EXPENSES_ROADVEH_REVENUE.0
    | 1 << ExpensesType::EXPENSES_AIRCRAFT_REVENUE.0
    | 1 << ExpensesType::EXPENSES_SHIP_REVENUE.0;
/// Expenses types whose cost is an expense of the quarter.
const RUNNING_EXPENSES: u32 = 1 << ExpensesType::EXPENSES_TRAIN_RUN.0
    | 1 << ExpensesType::EXPENSES_ROADVEH_RUN.0
    | 1 << ExpensesType::EXPENSES_AIRCRAFT_RUN.0
    | 1 << ExpensesType::EXPENSES_SHIP_RUN.0
    | 1 << ExpensesType::EXPENSES_PROPERTY.0
    | 1 << ExpensesType::EXPENSES_LOAN_INTEREST.0;

/// `SubtractMoneyFromAnyCompany`: deduct costs of a command from the money of the company
/// whose finance record is `c` and whose index is `company`.
///
/// # Safety
/// `c` is the live finance record of `company`.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) unsafe fn subtract_money_from_any_company(
    f: &FinanceServices,
    c: *mut Finances,
    company: CompanyID,
    cost: i64,
    expenses_type: ExpensesType,
) {
    if cost == 0 {
        return;
    }
    assert_with(|| expenses_type != ExpensesType::INVALID_EXPENSES);

    write_field!(c, money, read_field!(c, money).saturating_sub(cost));
    let expenses = usize::from(expenses_type.0);
    write_field!(
        c,
        yearly_expenses[0][expenses],
        read_field!(c, yearly_expenses[0][expenses]).saturating_add(cost)
    );

    if crate::bitmath_func::has_bit(REVENUE_EXPENSES, expenses_type.0) {
        write_field!(
            c,
            cur_economy.income,
            read_field!(c, cur_economy.income).saturating_sub(cost)
        );
    } else if crate::bitmath_func::has_bit(RUNNING_EXPENSES, expenses_type.0) {
        write_field!(
            c,
            cur_economy.expenses,
            read_field!(c, cur_economy.expenses).saturating_sub(cost)
        );
    }

    (f.invalidate_company_windows)(company.0);
}

/// `SubtractMoneyFromCompany`: subtract money from the `_current_company`, if the company is
/// valid.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
fn subtract_money_from_company(s: &Services, cost: i64, expenses_type: ExpensesType) {
    let company = CompanyID((s.shared.current_company)());
    let c = (s.finance.company_finances)(company.0);
    if !c.is_null() {
        // SAFETY: The pool returned the live record of `company`.
        unsafe { subtract_money_from_any_company(s.finance, c, company, cost, expenses_type) };
    }
}

/// `SubtractMoneyFromCompanyFract`: subtract money from a company, including the money
/// fraction. `company` is the company paying the bill, `cost` and `expenses_type` the cost of
/// a command.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "`c->money_fraction = m - (uint8_t)cost` (company_cmd.cpp:330)"
)]
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn subtract_money_from_company_fract(
    f: &FinanceServices,
    company: CompanyID,
    cost: i64,
    expenses_type: ExpensesType,
) {
    let c = (f.company_get_finances)(company.0);
    let m = read_field!(c, money_fraction);
    let mut cost = cost;

    write_field!(c, money_fraction, m.wrapping_sub(cost as u8));
    cost >>= 8;
    if read_field!(c, money_fraction) > m {
        cost = cost.saturating_add(1);
    }
    if cost != 0 {
        // SAFETY: `Company::Get` returned the live record of `company`.
        unsafe { subtract_money_from_any_company(f, c, company, cost, expenses_type) };
    }
}

/// `SubtractMoneyFromCompanyFract`, for the native function.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_subtract_fraction(
    f: &FinanceServices,
    company: u8,
    cost: i64,
    expenses_type: u8,
) {
    subtract_money_from_company_fract(f, CompanyID(company), cost, ExpensesType(expenses_type));
}

/// `UpdateLandscapingLimit`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "`limit = static_cast<uint32_t>(std::min<uint64_t>(limit + per_64k_frames, burst << 16))` (company_cmd.cpp:338)"
)]
fn update_landscaping_limit(limit: u32, per_64k_frames: u64, burst: u64) -> u32 {
    (u64::from(limit) + per_64k_frames).min(burst << 16) as u32
}

/// `UpdateLandscapingLimits`: update the landscaping limits per company.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn update_landscaping_limits(s: &Services) {
    let limits = s.shared.settings().construction_limits;
    companies(s, |c| {
        let c = c.finances;
        write_field!(
            c,
            terraform_limit,
            update_landscaping_limit(
                read_field!(c, terraform_limit),
                u64::from(read_field!(limits, terraform_per_64k_frames)),
                u64::from(read_field!(limits, terraform_frame_burst))
            )
        );
        write_field!(
            c,
            clear_limit,
            update_landscaping_limit(
                read_field!(c, clear_limit),
                u64::from(read_field!(limits, clear_per_64k_frames)),
                u64::from(read_field!(limits, clear_frame_burst))
            )
        );
        write_field!(
            c,
            tree_limit,
            update_landscaping_limit(
                read_field!(c, tree_limit),
                u64::from(read_field!(limits, tree_per_64k_frames)),
                u64::from(read_field!(limits, tree_frame_burst))
            )
        );
        write_field!(
            c,
            build_object_limit,
            update_landscaping_limit(
                read_field!(c, build_object_limit),
                u64::from(read_field!(limits, build_object_per_64k_frames)),
                u64::from(read_field!(limits, build_object_frame_burst))
            )
        );
    });
}

/// `CheckOwnership`: check whether the current owner owns something. If that isn't the case
/// an appropriate error will be given. `tile` is optional, to get the right town; if `tile`
/// is 0 then the owner can't be `OWNER_TOWN`. A succeeded command iff it's owned by the
/// current company, else a failed command.
pub(crate) fn check_ownership(s: &Services, out: Cost<'_>, owner: Owner, tile: TileIndex) {
    assert_with(|| owner.0 < OWNER_END);
    assert_with(|| owner != Owner::TOWN || tile.0 != 0);

    if owner.0 == (s.shared.current_company)() {
        out.zero();
        return;
    }

    out.error(STR_ERROR_OWNED_BY.0);
    if crate::command::is_local_company_of(s.shared.local_company, s.shared.current_company) {
        (out.1.encode_owned_by)(out.0, owner, tile);
        if owner != Owner::TOWN {
            (out.1.error_owner)(out.0, owner);
        }
    }
}

/// `CheckTileOwnership`: check whether the current owner owns the stuff on the given tile.
pub(crate) fn check_tile_ownership(s: &Services, out: Cost<'_>, tile: TileIndex) {
    check_ownership(
        s,
        out,
        crate::map_func::World::new(s.shared).owner(tile),
        tile,
    );
}

/// `CheckOwnership`, for the native function.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_check_ownership(
    s: &Services,
    out: *mut CommandCost,
    owner: Owner,
    tile: TileIndex,
) {
    check_ownership(s, Cost(out, s.cost), owner, tile);
}

/// `CheckTileOwnership`, for the native function.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_check_tile_ownership(
    s: &Services,
    out: *mut CommandCost,
    tile: TileIndex,
) {
    check_tile_ownership(s, Cost(out, s.cost), tile);
}

/// `Startup::stage`: the frame has not started.
pub const STARTUP_BEGIN: u8 = 0;
/// `Startup::stage`: `AI::StartNew` ran for the new company; its events follow.
pub const STARTUP_AI_STARTED: u8 = 1;
/// `Startup::stage`: the startup has ended.
pub const STARTUP_DONE: u8 = 2;

/// The caller-owned frame of `DoStartupNewCompany(is_ai, company)`. `index` is the new
/// company's index, or `u32::MAX` when none was created; `created` is its record, or null.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Startup {
    /// Is an AI company?
    pub is_ai: bool,
    /// `CompanyID` to use for the new company.
    pub company: CompanyID,
    /// One of the `STARTUP_*` stages.
    pub stage: u8,
    /// The index of the company created.
    pub index: u32,
    /// The company created, or null: the `Company *` the original returns.
    pub created: *mut OpenTTDCompany,
}

impl Startup {
    /// The frame of `DoStartupNewCompany(is_ai, company)`.
    pub(crate) const fn new(is_ai: bool, company: CompanyID) -> Self {
        Self {
            is_ai,
            company,
            stage: STARTUP_BEGIN,
            index: u32::MAX,
            created: core::ptr::null_mut(),
        }
    }
}

/// `DoStartupNewCompany`: create a new company and sets all company variables default values.
/// `true` asks the caller to run `AI::StartNew(frame.index)` and call again with the same
/// frame; afterwards the frame names the company created, if any.
#[allow(
    clippy::cast_possible_truncation,
    reason = "The startup frame carries the new company's `CompanyID` widened to `uint32_t`"
)]
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn do_startup_new_company(s: &Services, frame: &mut Startup) -> bool {
    if frame.stage == STARTUP_AI_STARTED {
        frame.stage = STARTUP_DONE;
        let index = CompanyID(frame.index as u8);
        crate::ai::broadcast_new_event(&ScriptEvent::company_new(index), index);
        crate::game::new_event(&ScriptEvent::company_new(index));
        return false;
    }
    frame.stage = STARTUP_DONE;

    if !(s.can_allocate_company)() {
        return false;
    }

    // we have to generate colour before this company is valid
    let colour = crate::company_identity::generate_company_colour(s.identity);

    let c = if frame.company == CompanyID::INVALID {
        (s.allocate_company)(CompanyID::INVALID, frame.is_ai)
    } else {
        if (s.shared.company_valid)(frame.company.0) {
            return false;
        }
        (s.allocate_company)(frame.company, frame.is_ai)
    };
    let c = CompanyRef::new(s.identity, c);
    frame.index = u32::from(c.index.0);
    frame.created = c.company;

    (s.identity.set_colour)(c.company, colour);

    crate::company_identity::reset_company_livery(s.identity, c.company);
    crate::company_identity::set_company_colour(c.index, (s.identity.colour)(c.company));

    // Scale the initial loan based on the inflation rounded down to the loan interval. The
    // maximum loan has already been inflation adjusted.
    let loan = ((INITIAL_LOAN
        .cast_unsigned()
        .wrapping_mul(read_field!(economy(), inflation_prices))
        >> 16)
        .cast_signed()
        / i64::from(LOAN_INTERVAL)
        * i64::from(LOAN_INTERVAL))
    .min(read_field!(economy(), max_loan));
    write_field!(c.finances, current_loan, loan);
    write_field!(c.finances, money, loan);

    let types = crate::type_availability::services(s.shared);
    (s.set_avail_railtypes)(
        c.company,
        crate::rail::get_company_rail_types(types, c.index, true),
    );
    (s.set_avail_roadtypes)(
        c.company,
        crate::road::availability::get_company_road_types(types, c.index, true),
    );
    (s.set_inaugurated_year)(c.company, crate::timer_game_common::economy_year());
    (s.set_inaugurated_year_calendar)(c.company, crate::timer_game_common::calendar_year());

    // If starting a player company in singleplayer and a favourite company manager face is
    // selected, choose it. Otherwise, use a random face. In a network game, we'll choose the
    // favourite face later in `cmd_company_ctrl` to sync it to all clients.
    let mut randomise_face = true;
    if !frame.is_ai && !(s.finance.networking)() && (s.parse_company_manager_face)(c.company) {
        randomise_face = false;
    }
    if randomise_face {
        crate::company_manager_face::randomise_company_manager_face(
            s.identity.faces,
            (s.identity.face)(c.company),
            s.identity.randomizer,
        );
    }

    (s.set_default_company_settings)(c.index);
    // SAFETY: The process-lifetime engine table.
    crate::engine::clear_engines_hidden_flag_of_company(unsafe { &*(s.engines)() }, c.index);

    crate::company_identity::generate_president_name(s.identity, c.company);

    set_window_dirty(WindowClass::WC_GRAPH_LEGEND, WindowNumber(0));
    invalidate_window_data(WindowClass::WC_CLIENT_LIST, WindowNumber(0), 0, false);
    invalidate_window_data(WindowClass::WC_LINKGRAPH_LEGEND, WindowNumber(0), 0, false);
    (s.identity.build_owner_legend)();
    invalidate_window_data(WindowClass::WC_SMALLMAP, WindowNumber(0), 1, false);

    if frame.is_ai && (!(s.finance.networking)() || (s.network_server)()) {
        frame.stage = STARTUP_AI_STARTED;
        return true;
    }

    crate::ai::broadcast_new_event(&ScriptEvent::company_new(c.index), c.index);
    crate::game::new_event(&ScriptEvent::company_new(c.index));
    false
}

/// `DoStartupNewCompany`, for the native function's loop around `AI::StartNew`. If that
/// throws, the frame is simply abandoned.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_startup_abi(s: &Services, frame: &mut Startup) -> u8 {
    u8::from(do_startup_new_company(s, frame))
}

/// The body of `_new_competitor_timeout`: start a new competitor company if possible. `true`
/// asks the caller to send the command to all clients that starts up a new AI.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn new_competitor_timeout(s: &Services) -> bool {
    let settings = s.shared.settings();
    if crate::session::mode() == GameMode::GM_MENU || !(s.ai_can_start_new)() {
        return false;
    }
    // SAFETY: The catalog's addresses of live settings, here and below.
    if (s.finance.networking)()
        && (s.company_count)() >= usize::from(unsafe { settings.max_companies.read() })
    {
        return false;
    }
    if unsafe { settings.competitor_interval.read() } == 0 {
        return false;
    }

    // count number of competitors
    let mut n = 0_u8;
    companies(s, |c| {
        if (s.identity.is_ai)(c.company) {
            n = n.wrapping_add(1);
        }
    });

    if n >= unsafe { settings.max_competitors.read() } {
        return false;
    }

    // Send a command to all clients to start up a new AI.
    // Works fine for Multiplayer and Singleplayer
    true
}

/// `_new_competitor_timeout`'s callback; `true` asks the caller to `Post` `CCA_NEW_AI`.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_competitor_timeout_abi(s: &Services) -> u8 {
    u8::from(new_competitor_timeout(s))
}

/// `group_all[type].num_vehicle` of the company `c`.
pub(crate) fn num_vehicle(s: &Services, c: *mut OpenTTDCompany, kind: VehicleType) -> u16 {
    crate::fleet::num_vehicle((s.group_all)(c, kind))
}

/// `CheckTakeoverVehicleLimit`: can company `cbig` buy company `csmall` without exceeding
/// vehicle limits?
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn check_takeover_vehicle_limit(
    s: &Services,
    cbig: CompanyID,
    csmall: CompanyID,
) -> bool {
    let c1 = (s.identity.company)(cbig);
    let c2 = (s.identity.company)(csmall);
    let settings = s.shared.settings();
    let within = |kind: VehicleType, max: u16| {
        i32::from(num_vehicle(s, c1, kind)) + i32::from(num_vehicle(s, c2, kind)) <= i32::from(max)
    };

    // Do the combined vehicle counts stay within the limits?
    // SAFETY: The catalog's addresses of live settings.
    within(VehicleType::VEH_TRAIN, unsafe {
        settings.max_trains.read()
    }) && within(VehicleType::VEH_ROAD, unsafe {
        settings.max_roadveh.read()
    }) && within(VehicleType::VEH_SHIP, unsafe { settings.max_ships.read() })
        && within(VehicleType::VEH_AIRCRAFT, unsafe {
            settings.max_aircraft.read()
        })
}

/// `CheckTakeoverVehicleLimit`, for the native function.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_takeover_allowed_abi(
    s: &Services,
    cbig: CompanyID,
    csmall: CompanyID,
) -> u8 {
    u8::from(check_takeover_vehicle_limit(s, cbig, csmall))
}

/// `HandleBankruptcyTakeover`: handle the bankruptcy take over of a company. Companies going
/// bankrupt will ask the other companies in order of their performance rating, so better
/// performing companies get the 'do you want to merge with Y' question earlier. The question
/// will then stay till either the company has gone bankrupt or got merged with a company.
#[allow(
    clippy::cast_possible_truncation,
    reason = "`c->bankrupt_timeout = TAKE_OVER_TIMEOUT` (company_cmd.cpp:761) stores the `int` constant in the `int16_t` member"
)]
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
fn handle_bankruptcy_takeover(s: &Services, c: CompanyRef) {
    // Amount of time out for each company to take over a company;
    // Timeout is a quarter (3 months of 30 days) divided over the
    // number of companies. The minimum number of days in a quarter
    // is 90: 31 in January, 28 in February and 31 in March.
    // Note that the company going bankrupt can't buy itself.
    const TAKE_OVER_TIMEOUT: i32 = 3 * 30 * DAY_TICKS / (MAX_COMPANIES as i32 - 1);

    let f = c.finances;
    assert_with(|| read_field!(f, bankrupt_asked).any());

    // We're currently asking some company to buy 'us'
    if read_field!(f, bankrupt_timeout) != 0 {
        write_field!(
            f,
            bankrupt_timeout,
            read_field!(f, bankrupt_timeout).wrapping_sub(i16::from(MAX_COMPANIES))
        );
        if read_field!(f, bankrupt_timeout) > 0 {
            return;
        }
        write_field!(f, bankrupt_timeout, 0);

        return;
    }

    // Did we ask everyone for bankruptcy? If so, bail out.
    if read_field!(f, bankrupt_asked).all() {
        return;
    }

    let mut best: Option<CompanyID> = None;
    let mut best_performance = -1_i32;

    // Ask the company with the highest performance history first
    companies(s, |c2| {
        let performance = read_field!(c2.finances, old_economy[1].performance_history);
        if read_field!(c2.finances, bankrupt_asked).none() // Don't ask companies going bankrupt themselves
            && !read_field!(f, bankrupt_asked).test(c2.index)
            && best_performance < performance
            && check_takeover_vehicle_limit(s, c2.index, c.index)
        {
            best_performance = performance;
            best = Some(c2.index);
        }
    });

    // Asked all companies? `best` is set exactly when `best_performance` is not -1.
    let Some(best) = best else {
        let mut asked = read_field!(f, bankrupt_asked);
        asked.set_all();
        write_field!(f, bankrupt_asked, asked);
        return;
    };

    let mut asked = read_field!(f, bankrupt_asked);
    asked.set(best);
    write_field!(f, bankrupt_asked, asked);

    write_field!(f, bankrupt_timeout, TAKE_OVER_TIMEOUT as i16);

    crate::ai::new_event(
        best,
        &ScriptEvent::company_ask_merger(c.index, read_field!(f, bankrupt_value)),
    );
    // `IsInteractiveCompany(best->index)`.
    if best.0 == (s.shared.local_company)() {
        (s.show_buy_company_dialog)(c.index, false);
    }
}

/// `OnTick_Companies`' stage: the company of the tick index is handled.
const TICK_BEGIN: u8 = 0;
/// `OnTick_Companies`' stage: posting competitors, the loop's next iteration pending.
const TICK_POSTING: u8 = 1;
/// `OnTick_Companies`' stage: the competitor timeout is randomized and reset.
const TICK_RESET: u8 = 2;
/// `OnTick_Companies`' stage: the tick index advances.
const TICK_END: u8 = 3;

/// The locals of `OnTick_Companies` that live across the native `CCA_NEW_AI` `Post`. The
/// settings are read on each pass of the loop, as the original reads them.
#[derive(Default)]
pub(crate) struct Tick {
    /// `num_ais`.
    num_ais: u8,
    /// `num_companies`.
    num_companies: usize,
    /// The loop's `i`.
    i: i32,
    /// `timeout`.
    timeout: i32,
    /// One of the `TICK_*` stages.
    stage: u8,
}

/// `OnTick_Companies`: called every tick for updating some company info. `true` asks the
/// caller to `Post` `CCA_NEW_AI` and call again with the same locals.
#[allow(
    clippy::cast_possible_truncation,
    reason = "`Company::GetIfValid(_cur_company_tick_index)` (company_cmd.cpp:774) converts the `uint` index, below `MAX_COMPANIES`, to `CompanyID`"
)]
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn on_tick_companies(s: &Services, t: &mut Tick) -> bool {
    let settings = s.shared.settings();
    if t.stage == TICK_BEGIN {
        if crate::session::mode() == GameMode::GM_EDITOR {
            return false;
        }

        // SAFETY: The serialized process-lifetime index.
        let index = CompanyID(unsafe { CUR_COMPANY_TICK_INDEX } as u8);
        if let Some(c) = CompanyRef::get_if_valid(s.identity, index) {
            if (s.identity.name_1)(c.company) != StringID(0) {
                crate::company_identity::generate_company_name(s.identity, c.company);
            }
            if read_field!(c.finances, bankrupt_asked).any() {
                handle_bankruptcy_takeover(s, c);
            }
        }

        t.stage = TICK_END;
        if (s.new_competitor_timeout_has_fired)()
            && crate::session::mode() != GameMode::GM_MENU
            && (s.ai_can_start_new)()
        {
            // SAFETY: The catalog's address of a live setting, here and below.
            t.timeout = i32::from(unsafe { settings.competitor_interval.read() })
                .wrapping_mul(60)
                .wrapping_mul(TICKS_PER_SECOND);
            t.stage = TICK_RESET;
            // If the interval is zero, start as many competitors as needed then check every
            // ~10 minutes if a company went bankrupt and needs replacing.
            if t.timeout == 0 {
                // count number of competitors
                t.num_ais = 0;
                companies(s, |cc| {
                    if (s.identity.is_ai)(cc.company) {
                        t.num_ais = t.num_ais.wrapping_add(1);
                    }
                });

                t.num_companies = (s.company_count)();
                t.i = 0;
                t.stage = TICK_POSTING;
            }
        }
    }

    if t.stage == TICK_POSTING {
        // One iteration of `for (auto i = 0; i < max_no_competitors; i++)` for each call, as
        // the caller posts between them; the increment of a resumed iteration has run before
        // this check.
        'iteration: {
            if t.i >= i32::from(unsafe { settings.max_competitors.read() }) {
                break 'iteration;
            }
            if (s.finance.networking)() {
                let num_companies = t.num_companies;
                t.num_companies = num_companies.wrapping_add(1);
                if num_companies >= usize::from(unsafe { settings.max_companies.read() }) {
                    break 'iteration;
                }
            }
            let num_ais = t.num_ais;
            t.num_ais = num_ais.wrapping_add(1);
            if num_ais >= unsafe { settings.max_competitors.read() } {
                break 'iteration;
            }
            t.i += 1;
            return true;
        }
        t.timeout = 10 * 60 * TICKS_PER_SECOND;
        t.stage = TICK_RESET;
    }

    if t.stage == TICK_RESET {
        // Randomize a bit when the AI is actually going to start; ranges from 87.5% .. 112.5%
        // of indicated value.
        let timeout = t.timeout;
        t.timeout = timeout
            .wrapping_add((s.script_random_next)((timeout / 4).cast_unsigned()).cast_signed())
            .wrapping_sub(timeout / 8);

        (s.new_competitor_timeout_reset)(t.timeout.max(1).cast_unsigned());
    }

    t.stage = TICK_END;
    // SAFETY: The serialized process-lifetime index.
    unsafe {
        CUR_COMPANY_TICK_INDEX = (CUR_COMPANY_TICK_INDEX + 1) % u32::from(MAX_COMPANIES);
    }
    false
}

/// The yearly company statistics timer body (`_economy_companies_yearly`): update the economic
/// data of all companies, and perhaps show the financial overview window of the local company.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn economy_companies_yearly(s: &Services) {
    // Copy statistics
    companies(s, |c| {
        let f = c.finances;
        // Move expenses to previous years.
        write_field!(f, yearly_expenses[2], read_field!(f, yearly_expenses[1]));
        write_field!(f, yearly_expenses[1], read_field!(f, yearly_expenses[0]));
        write_field!(
            f,
            yearly_expenses[0],
            [0; ExpensesType::EXPENSES_END.0 as usize]
        );
        invalidate_window_data(
            WindowClass::WC_FINANCES,
            WindowNumber::from(c.index),
            0,
            false,
        );
    });

    let settings = s.shared.settings();
    let local_company = CompanyID((s.shared.local_company)());
    // SAFETY: The catalog's addresses of live settings.
    if unsafe { settings.show_finances.read() } && local_company != CompanyID::SPECTATOR {
        (s.show_company_finances)(local_company);
        let c = CompanyRef::get(s.identity, local_company).finances;
        if read_field!(c, num_valid_stat_ent) > 5
            && read_field!(c, old_economy[0].performance_history)
                < read_field!(c, old_economy[4].performance_history)
        {
            if unsafe { settings.new_year.read() } {
                (s.snd_play_fx)(SoundFx::SND_01_BAD_YEAR);
            }
        } else if unsafe { settings.new_year.read() } {
            (s.snd_play_fx)(SoundFx::SND_00_GOOD_YEAR);
        }
    }
}

/// The network frame table, through the command post table.
fn network_frame(s: &Services) -> &'static crate::network_frame::Services {
    (((s.shared.command_post)().network)().network)()
}

crate::abi::shared_records! {
    fn abi_record;
    /// Arguments of `CmdCompanyCtrl`, in command order.
    #[derive(Clone, Copy)]
    "OpenTTDCmdCompanyCtrl" ["company"] => pub struct CmdCompanyCtrl {
        /// Action to perform.
        pub cca: CompanyCtrlAction,
        /// Company to perform the action on.
        pub company_id: CompanyID,
        /// The reason a company is removed.
        pub reason: CompanyRemoveReason,
        /// `ClientID`.
        pub client_id: ClientID,
    }
    /// One step of a pool's `Iterate(from)`: the item and its index, or null and `UINT32_MAX`
    /// past the last item.
    #[derive(Clone, Copy)]
    "OpenTTDCompanyPoolCursor" ["company"] => pub struct PoolCursor {
        /// The item's native shell.
        pub item: *mut c_void,
        /// `item->index`.
        pub index: u32,
    }
}

/// `for (T *item : T::Iterate())` over a pool cursor; the next item is looked up after `body`
/// returns, from the index after the visited one, so an item `body` deletes is not revisited.
fn pool_items(next: extern "C" fn(u32) -> PoolCursor, mut body: impl FnMut(PoolCursor)) {
    let mut from = 0_u32;
    loop {
        let cursor = next(from);
        if cursor.item.is_null() {
            return;
        }
        from = cursor.index + 1;
        body(cursor);
    }
}

/// `Control::stage`: the command has not started.
pub const CONTROL_BEGIN: u8 = 0;
/// `Control::stage`: `CCA_NEW_AI`'s `DoStartupNewCompany` is running in `startup`.
pub const CONTROL_STARTUP: u8 = 1;
/// `Control::stage`: the command has ended.
pub const CONTROL_DONE: u8 = 2;

/// The caller-owned frame of `CmdCompanyCtrl`: its arguments, its `DoCommandFlags` and the
/// `DoStartupNewCompany` frame of `CCA_NEW_AI`.
#[repr(C)]
pub struct Control {
    /// The command's arguments.
    pub args: CmdCompanyCtrl,
    /// `DoCommandFlags` bits.
    pub flags: u16,
    /// One of the `CONTROL_*` stages.
    pub stage: u8,
    /// `CCA_NEW_AI`'s new company.
    pub startup: Startup,
}

impl Control {
    /// The frame of a command with these flags and arguments.
    pub(crate) const fn new(flags: DoCommandFlags, args: CmdCompanyCtrl) -> Self {
        Self {
            args,
            flags: flags.0,
            stage: CONTROL_BEGIN,
            startup: Startup::new(true, CompanyID::INVALID),
        }
    }
}

/// `CmdCompanyCtrl`: control the companies: add, delete, etc. `true` asks the caller to run
/// `AI::StartNew(frame.startup.index)` and call again with the same frame; the command's
/// result is in `out` once it returns `false`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "The startup frame carries the new company's `CompanyID` widened to `uint32_t`"
)]
#[allow(clippy::too_many_lines)] // `CmdCompanyCtrl`'s four actions in one body, as the original.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn cmd_company_ctrl(
    s: &Services,
    ownership: &crate::tile_owner::Context,
    out: Cost<'_>,
    frame: &mut Control,
) -> bool {
    if frame.stage == CONTROL_BEGIN {
        let flags = DoCommandFlags(frame.flags);
        let CmdCompanyCtrl {
            cca,
            company_id,
            reason,
            client_id,
        } = frame.args;
        frame.stage = CONTROL_DONE;
        invalidate_window_data(WindowClass::WC_COMPANY_LEAGUE, WindowNumber(0), 0, false);

        match cca {
            // Create a new company
            CompanyCtrlAction::CCA_NEW => {
                // This command is only executed in a multiplayer game
                if !(s.finance.networking)() {
                    crate::command_func::cmd_error(out);
                    return false;
                }

                // Has the network client a correct ClientID?
                if !flags.test(DoCommandFlag::Execute) {
                    out.zero();
                    return false;
                }

                let ci =
                    crate::network_connection::ClientInfo::get_by_client_id(s.shared, client_id);

                // Delete multiplayer progress bar
                close_window_by_id(
                    WindowClass::WC_NETWORK_STATUS_WINDOW,
                    WindowNumber::from(WindowNumberEnum::WN_NETWORK_STATUS_WINDOW_JOIN),
                    true,
                    0,
                );

                // A human company never asks for `AI::StartNew`.
                let mut startup = Startup::new(false, CompanyID::INVALID);
                do_startup_new_company(s, &mut startup);

                // A new company could not be created, revert to being a spectator
                if startup.index == u32::MAX {
                    // We check for "ci != nullptr" as a client could have left by the time we
                    // execute this command.
                    if (s.network_server)() && !ci.is_null() {
                        let lifecycle = network_frame(s).lifecycle;
                        (lifecycle.set_info_company)(ci.as_ptr(), CompanyID::SPECTATOR.0);
                        (lifecycle.update_client_info)(ci.as_ptr());
                    }
                } else {
                    let index = CompanyID(startup.index as u8);
                    let c = startup.created;
                    (s.network_admin_company_new)(c);
                    let f = network_frame(s);
                    // SAFETY: `ci` is null or the pool's record of the client, which the call
                    // keeps.
                    unsafe { crate::network_server::new_company(f.lifecycle, f, c, ci) };

                    // This is the client (or non-dedicated server) who wants a new company
                    if client_id == crate::network_client::own_client_id(f.client) {
                        assert_with(|| (s.shared.local_company)() == CompanyID::SPECTATOR.0);
                        set_local_company(s, index);

                        // If a favourite company manager face is selected, choose it.
                        // Otherwise, use a random face. Because this needs to be synchronised
                        // over the network, only the client knows its configuration and we are
                        // currently in the execution of a command, we have to circumvent the
                        // normal ::Post logic for commands and just send the command.
                        (s.send_company_manager_face)(c);

                        // Now that we have a new company, broadcast our company settings to
                        // all clients so everything is in sync
                        (s.sync_company_settings)();

                        (s.shared.screen_dirty)();
                    }
                }
            }

            // Make a new AI company
            CompanyCtrlAction::CCA_NEW_AI => {
                if company_id != CompanyID::INVALID && company_id.0 >= MAX_COMPANIES {
                    crate::command_func::cmd_error(out);
                    return false;
                }

                // For network games, company deletion is delayed.
                if !(s.finance.networking)()
                    && company_id != CompanyID::INVALID
                    && (s.shared.company_valid)(company_id.0)
                {
                    crate::command_func::cmd_error(out);
                    return false;
                }

                if !flags.test(DoCommandFlag::Execute) {
                    out.zero();
                    return false;
                }

                // For network game, just assume deletion happened.
                assert_with(|| {
                    company_id == CompanyID::INVALID || !(s.shared.company_valid)(company_id.0)
                });

                frame.startup = Startup::new(true, company_id);
                frame.stage = CONTROL_STARTUP;
            }

            // Delete a company
            CompanyCtrlAction::CCA_DELETE => {
                if reason >= CompanyRemoveReason::CRR_END {
                    crate::command_func::cmd_error(out);
                    return false;
                }

                // We can't delete the last existing company in singleplayer mode.
                if !(s.finance.networking)() && (s.company_count)() == 1 {
                    crate::command_func::cmd_error(out);
                    return false;
                }

                let Some(c) = CompanyRef::get_if_valid(s.identity, company_id) else {
                    crate::command_func::cmd_error(out);
                    return false;
                };

                if !flags.test(DoCommandFlag::Execute) {
                    out.zero();
                    return false;
                }

                // Show the bankrupt news
                let cni =
                    CompanyNewsInformation::new(STR_NEWS_COMPANY_BANKRUPT_TITLE, c.company, None);
                let headline = get_encoded_string(
                    STR_NEWS_COMPANY_BANKRUPT_DESCRIPTION,
                    &[StringParameterData::String(&cni.company_name())],
                );
                add_company_news_item(&headline, cni);

                // Remove the company
                change_ownership_of_company_items(s, ownership, c.index, Owner::INVALID);
                if (s.identity.is_ai)(c.company) {
                    (s.ai_stop)(c.index);
                }

                let c_index = c.index;
                (s.delete_company)(c.company);
                crate::ai::broadcast_new_event(
                    &ScriptEvent::company_bankrupt(c_index),
                    CompanyID::INVALID,
                );
                crate::game::new_event(&ScriptEvent::company_bankrupt(c_index));
                (s.company_admin_remove)(c_index, reason);

                let objectives = (s.objectives)();
                if (objectives.page_count)() == 0 || (objectives.goal_count)() == 0 {
                    invalidate_window_data(WindowClass::WC_MAIN_TOOLBAR, WindowNumber(0), 0, false);
                }
                invalidate_window_data(WindowClass::WC_CLIENT_LIST, WindowNumber(0), 0, false);
            }

            _ => {
                crate::command_func::cmd_error(out);
                return false;
            }
        }
    }

    if frame.stage == CONTROL_STARTUP {
        if do_startup_new_company(s, &mut frame.startup) {
            return true;
        }
        frame.stage = CONTROL_DONE;
        if frame.startup.index != u32::MAX {
            let c = frame.startup.created;
            (s.network_admin_company_new)(c);
            let f = network_frame(s);
            // SAFETY: No client: `nullptr`.
            unsafe {
                crate::network_server::new_company(
                    f.lifecycle,
                    f,
                    c,
                    crate::network_connection::ClientInfo::NONE,
                );
            }
        }
    }

    invalidate_window_classes_data(WindowClass::WC_GAME_OPTIONS, 0, false);
    invalidate_window_classes_data(WindowClass::WC_SCRIPT_SETTINGS, 0, false);
    invalidate_window_classes_data(WindowClass::WC_SCRIPT_LIST, 0, false);

    out.zero();
    false
}

/// `CmdCompanyCtrl`, for the native facade's loop around `AI::StartNew`. If that throws, the
/// frame is simply abandoned.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_control_abi(
    s: &Services,
    ownership: &crate::tile_owner::Context,
    out: *mut CommandCost,
    frame: &mut Control,
) -> u8 {
    u8::from(cmd_company_ctrl(s, ownership, Cost(out, s.cost), frame))
}

/// `ExecuteAllowListCtrlAction`.
fn execute_allow_list_ctrl_action(
    s: &Services,
    action: CompanyAllowListCtrlAction,
    c: *mut OpenTTDCompany,
    public_key: ByteSpan,
) -> bool {
    match action {
        CompanyAllowListCtrlAction::CALCA_ADD => (s.allow_list_add)(c, public_key),
        CompanyAllowListCtrlAction::CALCA_REMOVE => (s.allow_list_remove)(c, public_key),
        _ => not_reached(),
    }
}

/// Arguments of `CmdCompanyAllowListCtrl`, in command order.
#[derive(Clone, Copy)]
pub(crate) struct CmdCompanyAllowListCtrlArgs {
    /// The action to perform.
    pub action: CompanyAllowListCtrlAction,
    /// The public key of the client to add or remove, borrowed for the command.
    pub public_key: ByteSpan,
}

/// `CmdCompanyAllowListCtrl`: add or remove the given public key to the allow list of this
/// company.
pub(crate) fn cmd_company_allow_list_ctrl(
    s: &Services,
    out: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdCompanyAllowListCtrlArgs,
) {
    let CmdCompanyAllowListCtrlArgs { action, public_key } = args;
    let current = CompanyID((s.shared.current_company)());
    let c = (s.identity.company_if_valid)(current);
    if c.is_null() {
        crate::command_func::cmd_error(out);
        return;
    }

    // The public key length includes the '\0'.
    if public_key.size != crate::network_type::NETWORK_PUBLIC_KEY_LENGTH - 1 {
        crate::command_func::cmd_error(out);
        return;
    }

    match action {
        CompanyAllowListCtrlAction::CALCA_ADD | CompanyAllowListCtrlAction::CALCA_REMOVE => {}
        _ => {
            crate::command_func::cmd_error(out);
            return;
        }
    }

    if flags.test(DoCommandFlag::Execute)
        && execute_allow_list_ctrl_action(s, action, c, public_key)
    {
        invalidate_window_data(WindowClass::WC_CLIENT_LIST, WindowNumber(0), 0, false);
        set_window_dirty(WindowClass::WC_COMPANY, WindowNumber::from(current));
    }

    out.zero();
}

/// `CmdCompanyAllowListCtrl`, for the native facade.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_allow_list(
    s: &Services,
    out: *mut CommandCost,
    flags: u16,
    action: u8,
    public_key: ByteSpan,
) {
    cmd_company_allow_list_ctrl(
        s,
        Cost(out, s.cost),
        DoCommandFlags(flags),
        CmdCompanyAllowListCtrlArgs {
            action: CompanyAllowListCtrlAction(action),
            public_key,
        },
    );
}

/// `CompanyServiceInterval`: get the service interval for the given company and vehicle type;
/// `None` for the client-default settings.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn company_service_interval(
    settings: &crate::settings_table::Services,
    identity: &crate::company_identity::Services,
    c: Option<*mut OpenTTDCompany>,
    kind: VehicleType,
) -> i32 {
    let vds = match c {
        None => (settings.client_vehicle_defaults)(),
        Some(c) => (identity.vehicle_defaults)(c),
    };
    i32::from(match kind {
        VehicleType::VEH_TRAIN => read_field!(vds, servint_trains),
        VehicleType::VEH_ROAD => read_field!(vds, servint_roadveh),
        VehicleType::VEH_AIRCRAFT => read_field!(vds, servint_aircraft),
        VehicleType::VEH_SHIP => read_field!(vds, servint_ships),
        _ => not_reached(),
    })
}

/// `CompanyServiceInterval`, for the native function; `c` is null for the client defaults.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_service_interval(
    s: &Services,
    c: *mut OpenTTDCompany,
    kind: VehicleType,
) -> i32 {
    company_service_interval(
        (s.settings)(),
        s.identity,
        (!c.is_null()).then_some(c),
        kind,
    )
}

/// Arguments of `CmdGiveMoney`, in command order.
#[derive(Clone, Copy)]
pub(crate) struct CmdGiveMoney {
    /// The amount of money to transfer; max 20.000.000.
    pub money: i64,
    /// The company to transfer the money to.
    pub dest_company: CompanyID,
}

/// `CmdGiveMoney`: transfer funds (money) from one company to another. To prevent abuse in
/// multiplayer games you can only send money to other companies if you have paid off your
/// loan (either explicitly, or implicitly given the fact that you have more money than loan).
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn cmd_give_money(
    s: &Services,
    out: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdGiveMoney,
) {
    let CmdGiveMoney {
        money,
        dest_company,
    } = args;
    // SAFETY: The catalog's address of a live setting.
    if !unsafe { s.shared.settings().give_money.read() } {
        crate::command_func::cmd_error(out);
        return;
    }

    let current = CompanyID((s.shared.current_company)());
    let c = CompanyRef::get(s.identity, current).finances;
    let amount = money.min(20_000_000);

    // You can only transfer funds that is in excess of your loan
    if read_field!(c, money).saturating_sub(read_field!(c, current_loan)) < amount || amount < 0 {
        out.error(STR_ERROR_INSUFFICIENT_FUNDS.0);
        return;
    }
    if !(s.shared.company_valid)(dest_company.0) {
        crate::command_func::cmd_error(out);
        return;
    }

    if flags.test(DoCommandFlag::Execute) {
        // Add money to company
        (s.finance.set_current_company)(dest_company.0);
        subtract_money_from_company(s, neg(amount), ExpensesType::EXPENSES_OTHER);
        (s.finance.set_current_company)(current.0);

        if (s.finance.networking)() {
            let dest_company_name =
                get_string(STR_COMPANY_NAME, [Integer(u64::from(dest_company.0))]);
            let from_company_name = get_string(STR_COMPANY_NAME, [Integer(u64::from(current.0))]);

            crate::network_internal::network_text_message(
                NetworkAction::NETWORK_ACTION_GIVE_MONEY,
                (s.draw_string_company_colour)(current),
                false,
                &from_company_name,
                &dest_company_name,
                Integer(amount.cast_unsigned()),
            );
        }
    }

    // Subtract money from local-company
    (s.cost.expense_money)(out.0, ExpensesType::EXPENSES_OTHER.0, amount);
}

/// `CmdGiveMoney`, for the native facade.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_give_money(
    s: &Services,
    out: *mut CommandCost,
    flags: u16,
    money: i64,
    dest_company: u8,
) {
    cmd_give_money(
        s,
        Cost(out, s.cost),
        DoCommandFlags(flags),
        CmdGiveMoney {
            money,
            dest_company: CompanyID(dest_company),
        },
    );
}

/// `GetFirstPlayableCompanyID`: get the index of the first available company. It attempts,
/// from first to last, and as soon as the attempt succeeds, to get the index of the company:
/// 1st - get the first existing human company.
/// 2nd - get the first non-existing company.
/// 3rd - get `CompanyID::Begin()`.
pub(crate) fn get_first_playable_company_id(s: &Services) -> CompanyID {
    let mut from = 0_u32;
    loop {
        let c = (s.identity.next_company)(from);
        if c.is_null() {
            break;
        }
        let index = (s.identity.index)(c);
        // `Company::IsHumanID(c->index)` resolves the company again.
        if !(s.identity.is_ai)((s.identity.company)(index)) {
            return index;
        }
        from = u32::from(index.0) + 1;
    }

    if (s.can_allocate_company)() {
        for c in CompanyID::BEGIN.0..MAX_COMPANIES {
            if !(s.shared.company_valid)(c) {
                return CompanyID(c);
            }
        }
    }

    CompanyID::BEGIN
}

/// `GetFirstPlayableCompanyID`, for the native function.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_first_playable(s: &Services) -> CompanyID {
    get_first_playable_company_id(s)
}

/// The station command services the station members are read and written through.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
fn station_commands(s: &Services) -> &'static crate::station_cmd::CommandServicesTable {
    // SAFETY: The process-lifetime station command table.
    unsafe { &*(s.shared.station_commands)() }
}

/// `for (const Station *st : Station::Iterate())`, visiting each station's native shell, as
/// its `BaseStation`, and its Rust service record.
fn stations(
    s: &Services,
    mut body: impl FnMut(*mut crate::station::BaseStation, *mut crate::station_service::Service),
) {
    s.shared
        .for_each_station(|st, service, _| body(st.cast_mut().cast(), service));
}

/// `for (const Vehicle *v : Vehicle::Iterate())`. The next vehicle is looked up after `body`
/// returns, from the index after the visited one, so vehicles `body` deletes are not visited.
fn vehicles(s: &Services, mut body: impl FnMut(VehicleRef)) {
    let r = crate::movement::table(s.shared).vehicle().newgrf();
    let mut from = 0_u32;
    while let Some(v) = (s.shared.vehicle_next)(from) {
        from = v.index(r).0 + 1;
        body(v);
    }
}

/// `CalculateCompanyAssetValue`: calculate the value of the assets of a company.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
fn calculate_company_asset_value(s: &Services, c: CompanyRef) -> i64 {
    let owner = c.index;

    let mut num = 0_u32;

    let commands = station_commands(s);
    // SAFETY: The process-lifetime station table.
    let world = unsafe { &*commands.station };
    stations(s, |st, _| {
        if (world.station_owner)(st) == owner.0 {
            num = num.wrapping_add(commands.facilities(st).0.count_ones());
        }
    });

    let mut value = i64::from(num)
        .saturating_mul(price(Price::PR_STATION_VALUE))
        .saturating_mul(25);

    let r = crate::movement::table(s.shared).vehicle().newgrf();
    vehicles(s, |v| {
        if v.owner() != owner {
            return;
        }

        let kind = v.vehicle_type();
        if kind == VehicleType::VEH_TRAIN
            || kind == VehicleType::VEH_ROAD
            || (kind == VehicleType::VEH_AIRCRAFT
                && crate::aircraft::is_normal_aircraft(v.subtype()))
            || kind == VehicleType::VEH_SHIP
        {
            value = value.saturating_add(v.value(r).saturating_mul(3) >> 1);
        }
    });

    value
}

/// `CalculateCompanyValue`: calculate the value of the company. That is the value of all
/// assets (vehicles, stations) and money (including loan), except when `including_loan` is
/// `false` which is useful when we want to calculate the value for bankruptcy.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn calculate_company_value(s: &Services, c: CompanyRef, including_loan: bool) -> i64 {
    let mut value = calculate_company_asset_value(s, c);

    // Add real money value
    if including_loan {
        value = value.saturating_sub(read_field!(c.finances, current_loan));
    }
    value = value.saturating_add(read_field!(c.finances, money));

    value.max(1)
}

/// `CalculateCompanyValue`, for the native function.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_value(
    s: &Services,
    c: *mut OpenTTDCompany,
    including_loan: bool,
) -> i64 {
    calculate_company_value(s, CompanyRef::new(s.identity, c), including_loan)
}

/// `CalculateHostileTakeoverValue`: calculate what you have to pay to take over a company.
///
/// This is different from bankruptcy and company value, and involves a few more parameters
/// to make it more realistic.
///
/// You have to pay for:
/// - The value of all the assets in the company.
/// - The loan the company has (the investors really want their money back).
/// - The profit for the next two years (if positive) based on the last four quarters.
///
/// And on top of that, they walk away with all the money they have in the bank.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
fn calculate_hostile_takeover_value(s: &Services, c: CompanyRef) -> i64 {
    let f = c.finances;
    let mut value = calculate_company_asset_value(s, c);

    value = value.saturating_add(read_field!(f, current_loan));
    // Negative balance is basically a loan.
    if read_field!(f, money) < 0 {
        value = value.saturating_add(neg(read_field!(f, money)));
    }

    for quarter in 0..4 {
        value = value.saturating_add(
            read_field!(f, old_economy[quarter].income)
                .saturating_add(read_field!(f, old_economy[quarter].expenses))
                .max(0)
                .saturating_mul(2),
        );
    }

    value.max(1)
}

/// `CalculateHostileTakeoverValue`, for the native function.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_hostile_takeover_value(
    s: &Services,
    c: *mut OpenTTDCompany,
) -> i64 {
    calculate_hostile_takeover_value(s, CompanyRef::new(s.identity, c))
}

/// `UpdateCompanyRatingAndValue`: if `update` is set to true, the economy is updated with
/// this score (also the house is updated, should only be true in the on-tick event). Returns
/// the actual score of this company.
#[allow(
    clippy::cast_possible_truncation,
    reason = "`s = Clamp<int64_t>(...) * _score_info[i].score / _score_info[i].needed` (economy.cpp:296) stores the `int64_t` product in the `int` score"
)]
#[allow(clippy::too_many_lines)] // `UpdateCompanyRatingAndValue`'s score parts in the original's order.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn update_company_rating_and_value(s: &Services, c: CompanyRef, update: bool) -> i32 {
    let owner = c.index;
    let f = c.finances;
    let mut score: i32;

    for i in ScoreID::SCORE_BEGIN.0..ScoreID::SCORE_END.0 {
        set_score_part(owner, ScoreID(i), 0);
    }

    // Count vehicles
    {
        let mut min_profit = 0_i64;
        let mut min_profit_first = true;
        let mut num = 0_u32;

        let vehicle = crate::movement::table(s.shared).vehicle();
        // SAFETY: The process-lifetime maintenance table.
        let maintenance = unsafe { &*vehicle.maintenance };
        vehicles(s, |v| {
            if v.owner() != owner {
                return;
            }
            if is_company_buildable_vehicle_type(v.vehicle_type()) && v.is_primary_vehicle() {
                let profit_last_year = v.profit_last_year(vehicle);
                if profit_last_year > 0 {
                    num = num.wrapping_add(1); // For the vehicle score only count profitable vehicles
                }
                if v.economy_age(maintenance) > VEHICLE_PROFIT_MIN_AGE {
                    // Find the vehicle with the lowest amount of profit
                    if min_profit_first || min_profit > v.profit_last_year(vehicle) {
                        min_profit = v.profit_last_year(vehicle);
                        min_profit_first = false;
                    }
                }
            }
        });

        min_profit >>= 8; // remove the fract part

        set_score_part(owner, ScoreID::SCORE_VEHICLES, i64::from(num));
        // Don't allow negative min_profit to show
        if min_profit > 0 {
            set_score_part(owner, ScoreID::SCORE_MIN_PROFIT, min_profit);
        }
    }

    // Count stations
    {
        let mut num = 0_u32;
        let commands = station_commands(s);
        // SAFETY: The process-lifetime station table.
        let world = unsafe { &*commands.station };
        stations(s, |st, service| {
            // Only count stations that are actually serviced
            if (world.station_owner)(st) == owner.0 {
                // SAFETY: The visited station's live service record; immediate copies.
                let activity = unsafe { crate::station_service::activity(service) };
                if activity.time_since_load <= 20 || activity.time_since_unload <= 20 {
                    num = num.wrapping_add(commands.facilities(st).0.count_ones());
                }
            }
        });
        set_score_part(owner, ScoreID::SCORE_STATIONS, i64::from(num));
    }

    // Generate statistics depending on recent income statistics
    {
        let numec = u32::from(read_field!(f, num_valid_stat_ent)).min(12) as usize;
        if numec != 0 {
            let mut min_income = i64::MAX;
            let mut max_income = i64::MIN;
            for i in 0..numec {
                let income = read_field!(f, old_economy[i].income)
                    .saturating_add(read_field!(f, old_economy[i].expenses));
                min_income = min_income.min(income);
                max_income = max_income.max(income);
            }

            if min_income > 0 {
                set_score_part(owner, ScoreID::SCORE_MIN_INCOME, min_income);
            }
            set_score_part(owner, ScoreID::SCORE_MAX_INCOME, max_income);
        }
    }

    // Generate score depending on amount of transported cargo
    {
        let numec = u32::from(read_field!(f, num_valid_stat_ent)).min(4) as usize;
        if numec != 0 {
            let mut total_delivered = 0_i64;
            for ce in 0..numec {
                for cargo in 0..usize::from(NUM_CARGO) {
                    total_delivered = total_delivered.saturating_add(i64::from(read_field!(
                        f,
                        old_economy[ce].delivered_cargo[cargo]
                    )));
                }
            }

            set_score_part(owner, ScoreID::SCORE_DELIVERED, total_delivered);
        }
    }

    // Generate score for variety of cargo
    {
        // SAFETY: The live finance record's `NUM_CARGO` delivered amounts; no callback runs.
        let num = unsafe {
            crate::cargo_type::cargo_array_count(
                (&raw const (*f).old_economy[0].delivered_cargo).cast(),
            )
        };
        set_score_part(owner, ScoreID::SCORE_CARGO, i64::from(num));
    }

    // Generate score for company's money
    {
        if read_field!(f, money) > 0 {
            set_score_part(owner, ScoreID::SCORE_MONEY, read_field!(f, money));
        }
    }

    // Generate score for loan
    {
        set_score_part(
            owner,
            ScoreID::SCORE_LOAN,
            i64::from(SCORE_INFO[usize::from(ScoreID::SCORE_LOAN.0)].needed)
                .saturating_sub(read_field!(f, current_loan)),
        );
    }

    // Now we calculate the score for each item..
    {
        let mut total_score = 0_i32;
        let mut s_i: i32;
        score = 0;
        for i in ScoreID::SCORE_BEGIN.0..ScoreID::SCORE_END.0 {
            // Skip the total
            if ScoreID(i) == ScoreID::SCORE_TOTAL {
                continue;
            }
            // Check the score
            let info = SCORE_INFO[usize::from(i)];
            s_i = (score_part(owner, ScoreID(i)).clamp(0, i64::from(info.needed))
                * i64::from(info.score)
                / i64::from(info.needed)) as i32;
            score += s_i;
            total_score += info.score;
        }

        set_score_part(owner, ScoreID::SCORE_TOTAL, i64::from(score));

        // We always want the score scaled to SCORE_MAX (1000)
        if total_score != SCORE_MAX {
            score = score * SCORE_MAX / total_score;
        }
    }

    if update {
        write_field!(f, old_economy[0].performance_history, score);
        crate::objects::update_company_hq(
            (s.identity.objects)(),
            (s.location_of_hq)(c.company),
            score.cast_unsigned(),
        );
        let value = calculate_company_value(s, c, true);
        write_field!(f, old_economy[0].company_value, value);
    }

    set_window_dirty(WindowClass::WC_PERFORMANCE_DETAIL, WindowNumber(0));
    score
}

/// `UpdateCompanyRatingAndValue`, for the native function.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_update_rating(
    s: &Services,
    c: *mut OpenTTDCompany,
    update: bool,
) -> i32 {
    update_company_rating_and_value(s, CompanyRef::new(s.identity, c), update)
}

/// `ChangeOwnershipOfCompanyItems`: change the ownership of all the items of a company.
/// `old_owner` is the company that gets removed, `new_owner` the company to merge to, or
/// `INVALID_OWNER` to remove the company.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "`t->ratings[old_owner] = RATING_INITIAL` (economy.cpp:381) stores in `int16_t`; `CompanyServiceInterval`'s `int` passes as `CmdChangeServiceInt`'s `uint16_t` interval (economy.cpp:453); the page cursor's index is a `StoryPageID`"
)]
#[allow(clippy::too_many_lines)] // `ChangeOwnershipOfCompanyItems`'s item walks in the original's order.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn change_ownership_of_company_items(
    s: &Services,
    ownership: &crate::tile_owner::Context,
    old_owner: Owner,
    new_owner: Owner,
) {
    // We need to set _current_company to old_owner before we try to move the client. This is
    // needed as it needs to know whether "you" really are the current local company.
    let cur_company = (s.shared.current_company)();
    (s.finance.set_current_company)(old_owner.0);
    // In all cases, make spectators of clients connected to that company
    if (s.finance.networking)() {
        (s.network_clients_to_spectators)(old_owner);
    }
    if old_owner.0 == (s.shared.local_company)() {
        // Single player cheated to AI company.
        // There are no spectators in singleplayer mode, so we must pick some other company.
        assert_with(|| !(s.finance.networking)());
        let cur_company2 = (s.shared.current_company)();
        let mut from = 0_u32;
        loop {
            let c = (s.identity.next_company)(from);
            if c.is_null() {
                break;
            }
            let index = (s.identity.index)(c);
            if index != old_owner {
                set_local_company(s, index);
                break;
            }
            from = u32::from(index.0) + 1;
        }
        (s.finance.set_current_company)(cur_company2);
        assert_with(|| old_owner.0 != (s.shared.local_company)());
    }

    assert_with(|| old_owner != new_owner);

    // Temporarily increase the company's money, to be sure that removing their property
    // doesn't fail because of lack of money. Not too drastically though, because it could
    // overflow
    if new_owner == Owner::INVALID {
        // `Money`'s constructor converts the `uint64_t` to `int64_t`.
        write_field!(
            CompanyRef::get(s.identity, old_owner).finances,
            money,
            (u64::MAX >> 2).cast_signed()
        ); // jackpot ;p
    }

    // The subsidy loop, and `RebuildSubsidisedSourceAndDestinationCache` after a deletion.
    // SAFETY: The process-lifetime subsidy table.
    unsafe { &*(s.subsidies)() }.transfer(old_owner, new_owner);

    // Take care of rating and transport rights in towns
    let mut from = 0_u32;
    loop {
        let id = (s.shared.town_next_id)(from);
        if id == u32::MAX {
            break;
        }
        from = id + 1;
        let t = (s.shared.town_owner_by_id)(id);
        let old = usize::from(old_owner.0);
        // If a company takes over, give the ratings to that company.
        if new_owner != Owner::INVALID {
            let new = usize::from(new_owner.0);
            let mut have_ratings = CompanyMask(read_field!(t, simulation.have_ratings));
            if have_ratings.test(old_owner) {
                if have_ratings.test(new_owner) {
                    // use max of the two ratings.
                    let rating = read_field!(t, simulation.ratings[new])
                        .max(read_field!(t, simulation.ratings[old]));
                    write_field!(t, simulation.ratings[new], rating);
                } else {
                    have_ratings.set(new_owner);
                    write_field!(t, simulation.have_ratings, have_ratings.0);
                    write_field!(
                        t,
                        simulation.ratings[new],
                        read_field!(t, simulation.ratings[old])
                    );
                }
            }
        }

        // Reset the ratings for the old owner
        write_field!(t, simulation.ratings[old], RATING_INITIAL as i16);
        let mut have_ratings = CompanyMask(read_field!(t, simulation.have_ratings));
        have_ratings.reset(old_owner);
        write_field!(t, simulation.have_ratings, have_ratings.0);

        // Transfer exclusive rights
        if read_field!(t, simulation.exclusive_counter) > 0
            && read_field!(t, simulation.exclusivity) == old_owner.0
        {
            if new_owner == Owner::INVALID {
                write_field!(t, simulation.exclusive_counter, 0);
                write_field!(t, simulation.exclusivity, CompanyID::INVALID.0);
            } else {
                write_field!(t, simulation.exclusivity, new_owner.0);
            }
        }
    }

    let vehicle = crate::movement::table(s.shared).vehicle();
    let groups = s.identity.groups;
    {
        vehicles(s, |v| {
            if v.owner() == old_owner && is_company_buildable_vehicle_type(v.vehicle_type()) {
                if new_owner == Owner::INVALID {
                    if v.previous().is_none() {
                        (vehicle.destroy)(v);
                    }
                } else {
                    if v.is_engine_countable(groups) {
                        crate::fleet::count_engine(groups, v, -1);
                    }
                    if v.is_primary_vehicle() {
                        crate::fleet::count_vehicle(groups, v, -1);
                    }
                }
            }
        });
    }

    // In all cases clear replace engine rules.
    // Even if it was copied, it could interfere with new owner's rules
    // `RemoveAllEngineReplacementForCompany(c)` is `RemoveAllEngineReplacement(&c->RenewalList())`.
    // SAFETY: The live company's renewal list; the group table's deletions end with the call.
    unsafe {
        crate::fleet::remove_all_engine_replacement(
            groups,
            (s.identity.renewal_head)((s.identity.company)(old_owner)),
        );
    }

    if new_owner == Owner::INVALID {
        crate::fleet::remove_all_groups_for_company(groups, old_owner);
    } else {
        let c = (s.identity.company)(new_owner);
        let mut from = 0_u32;
        loop {
            // SAFETY: The process-lifetime group table; the record ends with the step.
            let crate::fleet::NextGroup { group: g, index } = unsafe { (groups.next_group)(from) };
            if index == u32::MAX {
                break;
            }
            from = index + 1;
            if read_field!(g, owner) == old_owner.0 {
                write_field!(g, owner, new_owner.0);
                write_field!(g, number, (s.use_free_group)(c));
            }
        }
    }

    {
        let new_company = (new_owner != Owner::INVALID).then(|| (s.identity.company)(new_owner));
        let settings = (s.settings)();

        // Override company settings to new company defaults in case we need to convert them.
        // This is required as the CmdChangeServiceInt doesn't copy the supplied value when it
        // is non-custom
        if let Some(new_company) = new_company {
            let old_company = (s.identity.vehicle_defaults)((s.identity.company)(old_owner));
            let new_company = (s.identity.vehicle_defaults)(new_company);

            write_field!(
                old_company,
                servint_aircraft,
                read_field!(new_company, servint_aircraft)
            );
            write_field!(
                old_company,
                servint_trains,
                read_field!(new_company, servint_trains)
            );
            write_field!(
                old_company,
                servint_roadveh,
                read_field!(new_company, servint_roadveh)
            );
            write_field!(
                old_company,
                servint_ships,
                read_field!(new_company, servint_ships)
            );
            write_field!(
                old_company,
                servint_ispercent,
                read_field!(new_company, servint_ispercent)
            );
        }

        let r = vehicle.newgrf();
        vehicles(s, |v| {
            if v.owner() == old_owner && is_company_buildable_vehicle_type(v.vehicle_type()) {
                assert_with(|| new_owner != Owner::INVALID);
                let Some(new_company) = new_company else {
                    return;
                };

                // Correct default values of interval settings while maintaining custom set
                // ones. This prevents invalid values on mismatching company defaults being
                // accepted.
                if !v.service_interval_is_custom() {
                    // Technically, passing the interval is not needed as the command will
                    // query the default value itself. However, do not rely on that behaviour.
                    let interval = company_service_interval(
                        settings,
                        s.identity,
                        Some(new_company),
                        v.vehicle_type(),
                    );
                    let is_percent = read_field!(
                        (s.identity.vehicle_defaults)(new_company),
                        servint_ispercent
                    );
                    let transactions = settings.transactions;
                    // SAFETY: The nested result lives on this stack for the synchronous
                    // command; its result is dropped as the original drops it.
                    unsafe {
                        crate::command::with_cost(s.cost, |result| {
                            crate::command::nested(
                                transactions.commands,
                                s.shared,
                                result,
                                0,
                                DoCommandFlag::EXECUTE | DoCommandFlag::BANKRUPT,
                                |flags| {
                                    crate::vehicle_transactions::cmd_change_service_int(
                                        transactions,
                                        result.0,
                                        DoCommandFlags(flags),
                                        crate::vehicle_transactions::CmdChangeServiceInt {
                                            veh_id: v.index(r),
                                            serv_int: interval as u16,
                                            is_custom: false,
                                            is_percent,
                                        },
                                    );
                                },
                            );
                        });
                    }
                }

                v.set_owner(new_owner);

                // Owner changes, clear cache
                (vehicle.set_colourmap)(v, crate::sprites::PAL_NONE);
                // SAFETY: A live vehicle of the serialized game.
                unsafe { crate::newgrf_vehicle::invalidate_newgrf_cache(r, v) };

                if v.is_engine_countable(groups) {
                    crate::fleet::count_engine(groups, v, 1);
                }
                if v.is_primary_vehicle() {
                    crate::fleet::count_vehicle(groups, v, 1);
                    (s.set_unitnumber)(v, (s.use_free_unit)(new_company, v.vehicle_type()));
                }
            }
        });

        if new_owner != Owner::INVALID {
            crate::fleet::update_autoreplace(groups, new_owner);
        }
    }

    // Change ownership of tiles
    {
        let map_size = s.shared.map_size();
        for tile in 0..map_size {
            // SAFETY: The caller's frame owns the scratch of `ownership` across the traversal.
            unsafe {
                crate::tile_owner::change_tile_owner(
                    ownership,
                    TileIndex(tile),
                    old_owner,
                    new_owner,
                );
            }
        }

        let commands = s.identity.commands;
        let signals = crate::signal::Context {
            shared: commands.shared,
            rail: commands.rail_map,
            native: commands.signals,
        };
        if new_owner != Owner::INVALID {
            // Update all signals because there can be new segment that was owned by two
            // companies and signals were not propagated
            // Similar with crossings - it is needed to bar crossings that weren't before
            // because of different owner of crossing and approaching train
            // SAFETY: The process-lifetime train table.
            let leaves = unsafe { &*crate::movement::table(s.shared).train };
            let map_size = s.shared.map_size();
            let world = crate::map_func::World::new(s.shared);
            for tile in 0..map_size {
                let index = TileIndex(tile);
                if world.kind(index) == TileType::MP_RAILWAY
                    && world.is_tile_owner(index, new_owner)
                    && world.has_signals(index)
                {
                    let mut tracks = world.get_track_bits(index);
                    // there may be two tracks with signals for TRACK_BIT_HORZ and TRACK_BIT_VERT
                    loop {
                        let track = crate::track_func::remove_first_track(&mut tracks, |ok| {
                            crate::services_assert::check(ok);
                        });
                        if world.has_signal_on_track(index, track) {
                            signals.add_track_to_signal_buffer(index, track, new_owner);
                        }
                        if tracks == TrackBits::TRACK_BIT_NONE {
                            break;
                        }
                    }
                } else if world.level_crossing_tile(tile) && world.is_tile_owner(index, new_owner) {
                    crate::train::update_level_crossing(leaves, s.shared, index, true, false);
                }
            }
        }

        // update signals in buffer
        signals.update_signals_in_buffer();
    }

    // Add airport infrastructure count of the old company to the new one.
    if new_owner != Owner::INVALID {
        // The compound assignment evaluates the old company's count before the new one's.
        let airport = read_field!(
            crate::company_base::CompanyInfrastructure::get(s.shared, old_owner),
            airport
        );
        let infrastructure = crate::company_base::CompanyInfrastructure::get(s.shared, new_owner);
        write_field!(
            infrastructure,
            airport,
            read_field!(infrastructure, airport).wrapping_add(airport)
        );
    }

    let replacement = if new_owner == Owner::INVALID {
        Owner::NONE
    } else {
        new_owner
    };
    let commands = station_commands(s);
    // SAFETY: The process-lifetime station table.
    let world = unsafe { &*commands.station };
    // convert owner of stations (including deleted ones, but excluding buoys)
    stations(s, |st, _| {
        if (world.station_owner)(st) == old_owner.0 {
            // if a company goes bankrupt, set owner to OWNER_NONE so the sign doesn't
            // disappear immediately also, drawing station window would cause reading invalid
            // company's colour
            (commands.set_station_owner)(st, replacement.0);
        }
    });

    // do the same for waypoints (we need to do this here so deleted waypoints are converted
    // too)
    pool_items(s.next_waypoint, |wp| {
        let wp = wp.item.cast::<crate::station::BaseStation>();
        if (world.station_owner)(wp) == old_owner.0 {
            (commands.set_station_owner)(wp, replacement.0);
        }
    });

    pool_items(s.next_sign, |si| {
        let owner = (s.sign_owner)(si.item.cast());
        // SAFETY: The live sign's owner member, read and written in place.
        unsafe {
            if owner.read() == old_owner {
                owner.write(replacement);
            }
        }
    });

    // Remove Game Script created Goals, CargoMonitors and Story pages.
    let objectives = (s.objectives)();
    pool_items(s.next_goal, |g| {
        let g = g.item.cast::<crate::goal_type::Goal>();
        // SAFETY: The live goal's company member.
        if unsafe { (objectives.goal_company)(g).read() } == old_owner {
            (s.delete_goal)(g);
        }
    });

    crate::cargomonitor::clear_cargo_pickup_monitoring(old_owner);
    crate::cargomonitor::clear_cargo_delivery_monitoring(old_owner);

    pool_items(s.next_story_page, |sp| {
        let page = sp.item.cast::<crate::story_type::StoryPage>();
        // SAFETY: The live page's company member.
        if unsafe { (objectives.page_company)(page).read() } == old_owner {
            // `delete sp`: the page's elements first, as its destructor deletes them.
            // SAFETY: The page is live until its deletion below.
            unsafe {
                crate::objectives::delete_elements(
                    objectives,
                    crate::story_type::StoryPageID(sp.index as u16),
                );
            }
            (s.delete_story_page)(page);
        }
    });

    // Change colour of existing windows
    if new_owner != Owner::INVALID {
        (s.change_window_owner)(old_owner, new_owner);
    }

    (s.finance.set_current_company)(cur_company);

    (s.shared.screen_dirty)();
}

/// `ChangeOwnershipOfCompanyItems`, for the native function.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_change_ownership(
    s: &Services,
    ownership: &crate::tile_owner::Context,
    old_owner: Owner,
    new_owner: Owner,
) {
    change_ownership_of_company_items(s, ownership, old_owner, new_owner);
}

/// `CompanyCheckBankrupt`: check for bankruptcy of a company. Called every three months.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
fn company_check_bankrupt(s: &Services, c: CompanyRef) {
    let f = c.finances;
    // If "Infinite money" setting is on, companies should not go bankrupt.
    // SAFETY: The catalog's address of a live setting.
    if unsafe { s.shared.settings().infinite_money.read() } {
        return;
    }

    // If the company has money again, it does not go bankrupt
    if read_field!(f, money).saturating_sub(read_field!(f, current_loan)) >= neg(get_max_loan(f)) {
        let previous_months_of_bankruptcy =
            crate::math::ceil_div(u32::from(read_field!(f, months_of_bankruptcy)), 3).cast_signed();
        write_field!(f, months_of_bankruptcy, 0);
        write_field!(f, bankrupt_asked, CompanyMask::default());
        if previous_months_of_bankruptcy != 0 {
            (s.identity.admin_update)(c.company);
        }
        return;
    }

    write_field!(
        f,
        months_of_bankruptcy,
        read_field!(f, months_of_bankruptcy).wrapping_add(1)
    );

    match read_field!(f, months_of_bankruptcy) {
        // All the boring cases (months) with a bad balance where no action is taken
        0 | 1 | 2 | 3 | 5 | 6 | 8 | 9 => {}

        // Warn about bankruptcy after 3 months
        4 => {
            let cni =
                CompanyNewsInformation::new(STR_NEWS_COMPANY_IN_TROUBLE_TITLE, c.company, None);
            let headline = get_encoded_string(
                STR_NEWS_COMPANY_IN_TROUBLE_DESCRIPTION,
                &[StringParameterData::String(&cni.company_name())],
            );
            add_company_news_item(&headline, cni);
            crate::ai::broadcast_new_event(
                &ScriptEvent::company_in_trouble(c.index),
                CompanyID::INVALID,
            );
            crate::game::new_event(&ScriptEvent::company_in_trouble(c.index));
        }

        // Offer company for sale after 6 months
        7 => {
            // Don't consider the loan
            let val = calculate_company_value(s, c, false);

            write_field!(f, bankrupt_value, val);
            let mut asked = CompanyMask::default();
            asked.set(c.index); // Don't ask the owner
            write_field!(f, bankrupt_asked, asked);
            write_field!(f, bankrupt_timeout, 0);

            // The company assets should always have some value
            assert_with(|| read_field!(f, bankrupt_value) > 0);
        }

        // Bankrupt company after 6 months (if the company has no value) or latest after 9
        // months (if it still had value after 6 months)
        _ => {
            if !(s.finance.networking)() && (s.shared.local_company)() == c.index.0 {
                // If we are in singleplayer mode, leave the company playing. Eg. there is no
                // THE-END, otherwise mark the client as spectator to make sure they are no
                // longer in control of this company. However... when you join another
                // company (cheat) the "unowned" company can bankrupt.
                let mut asked = read_field!(f, bankrupt_asked);
                asked.set_all();
                write_field!(f, bankrupt_asked, asked);
            } else if !(s.finance.networking)() || (s.network_server)() {
                // Actually remove the company, but not when we're a network client. In case of
                // network clients we will be getting a command from the server. It is done in
                // this way as we are called from the StateGameLoop which can't change the
                // current company, and thus updating the local company triggers an assert
                // later on. In the case of a network game the command will be processed at a
                // time that changing the current company is okay. In case of single player we
                // are sure (the above check) that we are not the local company and thus we
                // won't be moved.
                crate::command_post::delete_company(s.shared, c.index);
                return;
            }
        }
    }

    // `CeilDiv(c->months_of_bankruptcy - 1, 3)` promotes the `uint8_t` to `int` before the
    // subtraction and converts the difference to `uint`.
    let months = read_field!(f, months_of_bankruptcy);
    if crate::math::ceil_div(u32::from(months), 3)
        != crate::math::ceil_div((i32::from(months) - 1).cast_unsigned(), 3)
    {
        (s.identity.admin_update)(c.company);
    }
}

/// `CompaniesGenStatistics`: update the finances of all companies. Pay for the stations,
/// update the history graph, update ratings and company values, and deal with bankruptcy.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
fn companies_gen_statistics(s: &Services) {
    // Check for bankruptcy each month
    companies(s, |c| company_check_bankrupt(s, c));

    let cur_company = (s.shared.current_company)();

    // Pay Infrastructure Maintenance, if enabled
    // SAFETY: The catalog's address of a live setting.
    if unsafe { s.shared.settings().infrastructure_maintenance.read() } {
        // Improved monthly infrastructure costs.
        companies(s, |c| {
            (s.finance.set_current_company)(c.index.0);

            let infrastructure = crate::company_base::CompanyInfrastructure::get(s.shared, c.index);
            let mut cost = 0_i64;
            // SAFETY: The live company's infrastructure; the totals end before the callbacks.
            let rail_total = unsafe {
                crate::company_base::CompanyInfrastructure::get_rail_total(infrastructure)
            };
            for rt in crate::rail_type::RAILTYPE_BEGIN..crate::rail_type::RAILTYPE_END {
                let rail = read_field!(infrastructure, rail[usize::from(rt)]);
                if rail != 0 {
                    cost = cost.saturating_add((s.rail_maintenance_cost)(
                        crate::rail_type::RailType(rt),
                        rail,
                        rail_total,
                    ));
                }
            }
            cost = cost.saturating_add((s.signal_maintenance_cost)(read_field!(
                infrastructure,
                signal
            )));
            // SAFETY: As `rail_total`.
            let road_total = unsafe {
                crate::company_base::CompanyInfrastructure::get_road_total(infrastructure)
            };
            let tram_total = unsafe {
                crate::company_base::CompanyInfrastructure::get_tram_total(infrastructure)
            };
            for rt in crate::road_type::ROADTYPE_BEGIN..crate::road_type::ROADTYPE_END {
                let road = read_field!(infrastructure, road[usize::from(rt)]);
                if road != 0 {
                    let rt = crate::road_type::RoadType(rt);
                    cost = cost.saturating_add((s.road_maintenance_cost)(
                        rt,
                        road,
                        if crate::road::road_type_is_road(rt) {
                            road_total
                        } else {
                            tram_total
                        },
                    ));
                }
            }
            cost = cost.saturating_add((s.canal_maintenance_cost)(read_field!(
                infrastructure,
                water
            )));
            cost = cost.saturating_add((s.station_maintenance_cost)(read_field!(
                infrastructure,
                station
            )));
            // SAFETY: The process-lifetime station lifecycle table.
            cost = cost.saturating_add(crate::station_lifecycle::airport_maintenance_cost(
                unsafe { &*(s.station_lifecycle)() },
                c.index,
            ));

            subtract_money_from_company(s, cost, ExpensesType::EXPENSES_PROPERTY);
        });
    }
    (s.finance.set_current_company)(cur_company);

    // Only run the economic statistics and update company stats every 3rd economy month (1st
    // of quarter).
    if !crate::bitmath_func::has_bit(
        1_u32 << 0 | 1 << 3 | 1 << 6 | 1 << 9,
        crate::timer_game_common::economy_month(),
    ) {
        return;
    }

    companies(s, |c| {
        let f = c.finances;
        // Drop the oldest history off the end
        for i in (1..MAX_HISTORY_QUARTERS).rev() {
            write_field!(f, old_economy[i], read_field!(f, old_economy[i - 1]));
        }
        write_field!(f, old_economy[0], read_field!(f, cur_economy));
        // SAFETY: All-zero is the record's value-initialised state.
        write_field!(f, cur_economy, unsafe { std::mem::zeroed() });

        if usize::from(read_field!(f, num_valid_stat_ent)) != MAX_HISTORY_QUARTERS {
            write_field!(
                f,
                num_valid_stat_ent,
                read_field!(f, num_valid_stat_ent) + 1
            );
        }

        update_company_rating_and_value(s, c, true);
        if read_field!(f, block_preview) != 0 {
            write_field!(f, block_preview, read_field!(f, block_preview) - 1);
        }
    });

    set_window_dirty(WindowClass::WC_INCOME_GRAPH, WindowNumber(0));
    set_window_dirty(WindowClass::WC_OPERATING_PROFIT, WindowNumber(0));
    set_window_dirty(WindowClass::WC_DELIVERED_CARGO, WindowNumber(0));
    set_window_dirty(WindowClass::WC_PERFORMANCE_HISTORY, WindowNumber(0));
    set_window_dirty(WindowClass::WC_COMPANY_VALUE, WindowNumber(0));
    set_window_dirty(WindowClass::WC_COMPANY_LEAGUE, WindowNumber(0));
}

/// `AddInflation`: add monthly inflation. `check_year`: shall the inflation get stopped after
/// 170 years? Returns true if inflation is maxed and nothing was changed.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn add_inflation(check_year: bool) -> bool {
    // The cargo payment inflation differs from the normal inflation, so the
    // relative amount of money you make with a transport decreases slowly over
    // the 170 years. After a few hundred years we reach a level in which the
    // games will become unplayable as the maximum income will be less than
    // the minimum running cost.
    //
    // Furthermore there are a lot of inflation related overflows all over the
    // place. Solving them is hardly possible because inflation will always
    // reach the overflow threshold some day. So we'll just perform the
    // inflation mechanism during the first 170 years (the amount of years that
    // one had in the original TTD) and stop doing the inflation after that
    // because it only causes problems that can't be solved nicely and the
    // inflation doesn't add anything after that either; it even makes playing
    // it impossible due to the diverging cost and income rates.
    let year = crate::timer_game_common::calendar_year().0;
    if check_year && !(ORIGINAL_BASE_YEAR..ORIGINAL_MAX_YEAR).contains(&year) {
        return true;
    }

    let e = economy();
    if read_field!(e, inflation_prices) == MAX_INFLATION
        || read_field!(e, inflation_payment) == MAX_INFLATION
    {
        return true;
    }

    // Approximation for (100 + infl_amount)% ** (1 / 12) - 100%
    // scaled by 65536
    // 12 -> months per year
    // This is only a good approximation for small values
    let prices = read_field!(e, inflation_prices);
    write_field!(
        e,
        inflation_prices,
        prices.wrapping_add(
            prices
                .wrapping_mul(u64::from(read_field!(e, infl_amount)))
                .wrapping_mul(54)
                >> 16
        )
    );
    let payment = read_field!(e, inflation_payment);
    write_field!(
        e,
        inflation_payment,
        payment.wrapping_add(
            payment
                .wrapping_mul(u64::from(read_field!(e, infl_amount_pr)))
                .wrapping_mul(54)
                >> 16
        )
    );

    if read_field!(e, inflation_prices) > MAX_INFLATION {
        write_field!(e, inflation_prices, MAX_INFLATION);
    }
    if read_field!(e, inflation_payment) > MAX_INFLATION {
        write_field!(e, inflation_payment, MAX_INFLATION);
    }

    false
}

/// `AddInflation`, for the native function.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_economy_add_inflation_abi(check_year: bool) -> u8 {
    u8::from(add_inflation(check_year))
}

/// `RecomputePrices`: computes all prices, payments and maximum loan.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn recompute_prices(s: &Services) {
    let settings = s.shared.settings();
    let e = economy();
    // Setup maximum loan as a rounded down multiple of LOAN_INTERVAL.
    // SAFETY: The catalog's addresses of live settings, here and below.
    write_field!(
        e,
        max_loan,
        ((u64::from(unsafe { settings.max_loan.read() })
            .wrapping_mul(read_field!(e, inflation_prices))
            >> 16)
            / u64::from(LOAN_INTERVAL.cast_unsigned())
            * u64::from(LOAN_INTERVAL.cast_unsigned()))
        .cast_signed()
    );

    // Setup price bases
    for i in Price::PR_BEGIN.0..Price::PR_END.0 {
        let i = Price(i);
        let mut price = (s.price_base_start_price)(i);

        // Apply difficulty settings
        let mut r#mod = 1_u32;
        match (s.price_base_category)(i) {
            PriceCategory::PCAT_RUNNING => {
                r#mod = u32::from(unsafe { settings.vehicle_costs.read() });
            }

            PriceCategory::PCAT_CONSTRUCTION => {
                r#mod = u32::from(unsafe { settings.construction_cost.read() });
            }

            _ => {}
        }
        match r#mod {
            0 => price = price.saturating_mul(6),
            1 => price = price.saturating_mul(8), // normalised to 1 below
            2 => price = price.saturating_mul(9),
            _ => not_reached(),
        }

        // Apply inflation
        price = price
            .cast_unsigned()
            .wrapping_mul(read_field!(e, inflation_prices))
            .cast_signed();

        // Apply newgrf modifiers, remove fractional part of inflation, and normalise on medium
        // difficulty.
        // SAFETY: Process-lifetime storage the game-state mutex serializes.
        let multiplier = unsafe {
            (&raw const PRICE_BASE_MULTIPLIER)
                .cast::<i8>()
                .add(usize::from(i.0))
                .read()
        };
        let shift = i32::from(multiplier) - 16 - 3;
        if shift >= 0 {
            price = price.wrapping_shl(shift.cast_unsigned());
        } else {
            price >>= -shift;
        }

        // Make sure the price does not get reduced to zero.
        // Zero breaks quite a few commands that use a zero cost to see whether something got
        // changed or not and based on that cause an error. When the price is zero that fails
        // even when things are done.
        if price == 0 {
            price = (s.price_base_start_price)(i).clamp(-1, 1);
            // No base price should be zero, but be sure.
            assert_with(|| price != 0);
        }
        // Store value
        set_price(i, price);
    }

    // Setup cargo payment
    for cs in crate::cargo_registry::cargo_specs() {
        let payment = (s.cargo_initial_payment)(cs)
            .saturating_mul(read_field!(e, inflation_payment).cast_signed())
            >> 16;
        (s.cargo_set_current_payment)(cs, payment);
    }

    set_window_classes_dirty(WindowClass::WC_BUILD_VEHICLE);
    set_window_classes_dirty(WindowClass::WC_REPLACE_VEHICLE);
    set_window_classes_dirty(WindowClass::WC_VEHICLE_DETAILS);
    set_window_classes_dirty(WindowClass::WC_COMPANY_INFRASTRUCTURE);
    invalidate_window_data(WindowClass::WC_PAYMENT_RATES, WindowNumber(0), 0, false);
}

/// `RecomputePrices`, for the native function.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_economy_recompute_prices(s: &Services) {
    recompute_prices(s);
}

/// `CompaniesPayInterest`: let all companies pay the monthly interest on their loan.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
fn companies_pay_interest(s: &Services) {
    let cur_company = (s.shared.current_company)();
    let e = economy();
    companies(s, |c| {
        (s.finance.set_current_company)(c.index.0);

        // Over a year the paid interest should be "loan * interest percentage",
        // but... as that number is likely not dividable by 12 (pay each month),
        // one needs to account for that in the monthly fee calculations.
        //
        // To easily calculate what one should pay "this" month, you calculate
        // what (total) should have been paid up to this month and you subtract
        // whatever has been paid in the previous months. This will mean one month
        // it'll be a bit more and the other it'll be a bit less than the average
        // monthly fee, but on average it will be exact.
        //
        // In order to prevent cheating or abuse (just not paying interest by not
        // taking a loan) we make companies pay interest on negative cash as well,
        // except if infinite money is enabled.
        let interest_rate = i64::from(read_field!(e, interest_rate));
        let mut yearly_fee =
            read_field!(c.finances, current_loan).saturating_mul(interest_rate) / 100;
        let available_money = get_available_money(s.shared, s.finance, c.index);
        if available_money < 0 {
            yearly_fee =
                yearly_fee.saturating_add(neg(available_money).saturating_mul(interest_rate) / 100);
        }
        let month = i64::from(crate::timer_game_common::economy_month());
        let up_to_previous_month = yearly_fee.saturating_mul(month) / 12;
        let up_to_this_month = yearly_fee.saturating_mul(month + 1) / 12;

        subtract_money_from_company(
            s,
            up_to_this_month.saturating_sub(up_to_previous_month),
            ExpensesType::EXPENSES_LOAN_INTEREST,
        );

        subtract_money_from_company(
            s,
            price(Price::PR_STATION_VALUE) >> 2,
            ExpensesType::EXPENSES_OTHER,
        );
    });
    (s.finance.set_current_company)(cur_company);
}

/// `HandleEconomyFluctuations`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "`_economy.fluct = -(int)GB(Random(), 0, 2)` and `GB(Random(), 0, 8) + 312` (economy.cpp:858-861) store in the `int16_t` member"
)]
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
fn handle_economy_fluctuations(s: &Services) {
    let e = economy();
    // SAFETY: The catalog's address of a live setting.
    if unsafe { s.shared.settings().economy.read() } {
        // When economy is Fluctuating, decrease counter
        write_field!(e, fluct, read_field!(e, fluct).wrapping_sub(1));
    } else if crate::economy_func::economy_is_in_recession() {
        // When it's Steady and we are in recession, end it now
        write_field!(e, fluct, -12);
    } else {
        // No need to do anything else in other cases
        return;
    }

    if read_field!(e, fluct) == 0 {
        write_field!(
            e,
            fluct,
            -(crate::bitmath_func::gb(s.shared.random(), 0, 2) as i16)
        );
        add_news_item(&AddNewsItemArgs::new(
            &get_encoded_string(STR_NEWS_BEGIN_OF_RECESSION, &[]),
            NewsType::Economy,
            NewsStyle::Normal,
            NewsFlags::default(),
        ));
    } else if read_field!(e, fluct) == -12 {
        write_field!(
            e,
            fluct,
            (crate::bitmath_func::gb(s.shared.random(), 0, 8) + 312) as i16
        );
        add_news_item(&AddNewsItemArgs::new(
            &get_encoded_string(STR_NEWS_END_OF_RECESSION, &[]),
            NewsType::Economy,
            NewsStyle::Normal,
            NewsFlags::default(),
        ));
    }
}

/// `ResetPriceBaseMultipliers`: reset changes to the price base multipliers.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn reset_price_base_multipliers() {
    // SAFETY: Process-lifetime storage the game-state mutex serializes.
    unsafe {
        PRICE_BASE_MULTIPLIER = [0; Price::PR_END.0 as usize];
    }
}

/// `ResetPriceBaseMultipliers`, for the native function.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_economy_reset_price_multipliers() {
    reset_price_base_multipliers();
}

/// `SetPriceBaseMultiplier`: change a price base by the given factor. The price base is
/// altered by factors of two. `NewBaseCost = OldBaseCost * 2^n`. `price` is the index of the
/// price base to change, `factor` the amount to change by.
#[allow(
    clippy::cast_possible_truncation,
    reason = "`_price_base_multiplier[price] = Clamp(factor, MIN_PRICE_MODIFIER, MAX_PRICE_MODIFIER)` (economy.cpp:885) stores in `int8_t`"
)]
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn set_price_base_multiplier(price: Price, factor: i32) {
    assert_with(|| price < Price::PR_END);
    // SAFETY: Process-lifetime storage the game-state mutex serializes.
    unsafe {
        (&raw mut PRICE_BASE_MULTIPLIER)
            .cast::<i8>()
            .add(usize::from(price.0))
            .write(factor.clamp(MIN_PRICE_MODIFIER, MAX_PRICE_MODIFIER) as i8);
    }
}

/// `SetPriceBaseMultiplier`, for the native function.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_economy_set_price_multiplier(price: u8, factor: i32) {
    set_price_base_multiplier(Price(price), factor);
}

/// `StartupEconomy`; `industry_builder` is the industry builder state, which
/// `StartupIndustryDailyChanges` updates.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "`_economy.infl_amount_pr = std::max(0, initial_interest - 1)` stores in `uint8_t` and `_economy.fluct = GB(Random(), 0, 8) + 168` in `int16_t` (economy.cpp:913-914)"
)]
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn startup_economy(s: &Services, industry_builder: *mut crate::industry::Builder) {
    let settings = s.shared.settings();
    let e = economy();
    // SAFETY: The catalog's addresses of live settings.
    let initial_interest = unsafe { settings.initial_interest.read() };
    write_field!(e, interest_rate, initial_interest);
    write_field!(e, infl_amount, initial_interest);
    write_field!(
        e,
        infl_amount_pr,
        (i32::from(initial_interest) - 1).max(0) as u8
    );
    write_field!(
        e,
        fluct,
        (crate::bitmath_func::gb(s.shared.random(), 0, 8) + 168) as i16
    );

    if unsafe { settings.inflation.read() } {
        // Apply inflation that happened before our game start year.
        let months = (crate::timer_game_common::calendar_year()
            .0
            .min(ORIGINAL_MAX_YEAR)
            - ORIGINAL_BASE_YEAR)
            * 12;
        for _ in 0..months {
            add_inflation(false);
        }
    }

    // Set up prices
    recompute_prices(s);

    // As we are starting a new game, initialize the counter too
    // SAFETY: The live builder state the caller passes.
    unsafe {
        crate::industry::startup_industry_daily_changes(
            industry_builder,
            s.shared.map().log_x + s.shared.map_log_y(),
            true,
        );
    }
}

/// `InitializeEconomy`: resets economy to initial values.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn initialize_economy() {
    let e = economy();
    write_field!(e, inflation_prices, 1 << 16);
    write_field!(e, inflation_payment, 1 << 16);
    crate::cargomonitor::clear_cargo_pickup_monitoring(Owner::INVALID);
    crate::cargomonitor::clear_cargo_delivery_monitoring(Owner::INVALID);
}

/// `InitializeEconomy`, for the native function.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_economy_initialize() {
    initialize_economy();
}

/// The calendar-month timer body (`_calendar_inflation_monthly`): every calendar month update
/// of inflation.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn calendar_inflation_monthly(s: &Services) {
    // SAFETY: The catalog's address of a live setting.
    if unsafe { s.shared.settings().inflation.read() } {
        add_inflation(true);
        recompute_prices(s);
    }
}

/// The economy-month timer body (`_economy_companies_monthly`): every economy month update of
/// company economic data, plus economy fluctuations.
pub(crate) fn economy_companies_monthly(s: &Services) {
    companies_gen_statistics(s);
    companies_pay_interest(s);
    handle_economy_fluctuations(s);
}

/// `DoAcquireCompany`.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
fn do_acquire_company(
    s: &Services,
    ownership: &crate::tile_owner::Context,
    c: CompanyRef,
    hostile_takeover: bool,
) {
    let ci = c.index;
    let current = CompanyID((s.shared.current_company)());

    let cni = CompanyNewsInformation::new(
        STR_NEWS_COMPANY_MERGER_TITLE,
        c.company,
        Some((s.identity.company)(current)),
    );
    let company_name = cni.company_name();
    let other_company_name = cni.other_company_name();
    let headline = if hostile_takeover {
        get_encoded_string(
            STR_NEWS_MERGER_TAKEOVER_TITLE,
            &[
                StringParameterData::String(&company_name),
                StringParameterData::String(&other_company_name),
            ],
        )
    } else {
        get_encoded_string(
            STR_NEWS_COMPANY_MERGER_DESCRIPTION,
            &[
                StringParameterData::String(&company_name),
                StringParameterData::String(&other_company_name),
                Integer(read_field!(c.finances, bankrupt_value).cast_unsigned()),
            ],
        )
    };
    add_company_news_item(&headline, cni);
    crate::ai::broadcast_new_event(
        &ScriptEvent::company_merger(ci, current),
        CompanyID::INVALID,
    );
    crate::game::new_event(&ScriptEvent::company_merger(ci, current));

    change_ownership_of_company_items(s, ownership, ci, current);

    if (s.identity.is_ai)(c.company) {
        (s.ai_stop)(c.index);
    }

    (s.close_company_windows)(ci);
    invalidate_window_classes_data(WindowClass::WC_TRAINS_LIST, 0, false);
    invalidate_window_classes_data(WindowClass::WC_SHIPS_LIST, 0, false);
    invalidate_window_classes_data(WindowClass::WC_ROADVEH_LIST, 0, false);
    invalidate_window_classes_data(WindowClass::WC_AIRCRAFT_LIST, 0, false);
    invalidate_window_data(WindowClass::WC_CLIENT_LIST, WindowNumber(0), 0, false);

    (s.delete_company)(c.company);
}

/// Arguments of `CmdBuyCompany`, in command order.
#[derive(Clone, Copy)]
pub(crate) struct CmdBuyCompany {
    /// Company to buy up.
    pub target_company: CompanyID,
    /// Whether to buy up the company even if it is not bankrupt.
    pub hostile_takeover: bool,
}

/// `CmdBuyCompany`: buy up another company. When a competing company is gone bankrupt you get
/// the chance to purchase that company.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn cmd_buy_company(
    s: &Services,
    ownership: &crate::tile_owner::Context,
    out: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdBuyCompany,
) {
    let CmdBuyCompany {
        target_company,
        mut hostile_takeover,
    } = args;
    let Some(c) = CompanyRef::get_if_valid(s.identity, target_company) else {
        crate::command_func::cmd_error(out);
        return;
    };
    let current = CompanyID((s.shared.current_company)());

    // If you do a hostile takeover but the company went bankrupt, buy it via bankruptcy rules.
    if hostile_takeover && read_field!(c.finances, bankrupt_asked).test(current) {
        hostile_takeover = false;
    }

    // Disable takeovers when not asked
    if !hostile_takeover && !read_field!(c.finances, bankrupt_asked).test(current) {
        crate::command_func::cmd_error(out);
        return;
    }

    // Only allow hostile takeover of AI companies and when in single player
    if hostile_takeover && !(s.identity.is_ai)(c.company) {
        crate::command_func::cmd_error(out);
        return;
    }
    if hostile_takeover && (s.finance.networking)() {
        crate::command_func::cmd_error(out);
        return;
    }

    // Disable taking over the local company in singleplayer mode
    if !(s.finance.networking)() && (s.shared.local_company)() == c.index.0 {
        crate::command_func::cmd_error(out);
        return;
    }

    // Do not allow companies to take over themselves
    if target_company == current {
        crate::command_func::cmd_error(out);
        return;
    }

    // Do not allow takeover if the resulting company would have too many vehicles.
    if !check_takeover_vehicle_limit(s, current, target_company) {
        out.error(STR_ERROR_TOO_MANY_VEHICLES_IN_GAME.0);
        return;
    }

    // Get the cost here as the company is deleted in DoAcquireCompany.
    // For bankruptcy this amount is calculated when the offer was made;
    // for hostile takeover you pay the current price.
    let cost = if hostile_takeover {
        calculate_hostile_takeover_value(s, c)
    } else {
        read_field!(c.finances, bankrupt_value)
    };
    (s.cost.expense_money)(out.0, ExpensesType::EXPENSES_OTHER.0, cost);

    if flags.test(DoCommandFlag::Execute) {
        do_acquire_company(s, ownership, c, hostile_takeover);
    }
}

/// `CmdBuyCompany`, for the native facade.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_buy(
    s: &Services,
    ownership: &crate::tile_owner::Context,
    out: *mut CommandCost,
    flags: u16,
    target_company: CompanyID,
    hostile_takeover: bool,
) {
    cmd_buy_company(
        s,
        ownership,
        Cost(out, s.cost),
        DoCommandFlags(flags),
        CmdBuyCompany {
            target_company,
            hostile_takeover,
        },
    );
}

/// Arguments of `CmdIncreaseLoan` and `CmdDecreaseLoan`, in command order.
#[derive(Clone, Copy)]
pub(crate) struct CmdLoan {
    /// How the amount is chosen: one `LOAN_INTERVAL`, the maximum permitting money (press
    /// CTRL), or `amount`.
    pub cmd: LoanCommand,
    /// Amount to change the loan with, multitude of `LOAN_INTERVAL`. Only used when `cmd` is
    /// `LoanCommand::Amount`.
    pub amount: i64,
}

/// `CmdIncreaseLoan`: increase the loan of your company.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn cmd_increase_loan(s: &Services, out: Cost<'_>, flags: DoCommandFlags, args: CmdLoan) {
    let CmdLoan { cmd, amount } = args;
    let current = CompanyID((s.shared.current_company)());
    let c = CompanyRef::get(s.identity, current).finances;
    let max_loan = get_max_loan(c);
    if read_field!(c, current_loan) >= max_loan {
        (s.cost.error_param)(
            out.0,
            STR_ERROR_MAXIMUM_PERMITTED_LOAN.0,
            max_loan.cast_unsigned(),
        );
        return;
    }

    let loan = match cmd {
        // Take some extra loan
        LoanCommand::INTERVAL => i64::from(LOAN_INTERVAL),
        // Take a loan as big as possible
        LoanCommand::MAX => max_loan.saturating_sub(read_field!(c, current_loan)),
        // Take the given amount of loan
        LoanCommand::AMOUNT => {
            let loan = amount;
            if loan < i64::from(LOAN_INTERVAL)
                || read_field!(c, current_loan).saturating_add(loan) > max_loan
                || loan % i64::from(LOAN_INTERVAL) != 0
            {
                crate::command_func::cmd_error(out);
                return;
            }
            loan
        }
        // Invalid method
        _ => {
            crate::command_func::cmd_error(out);
            return;
        }
    };

    // In case adding the loan triggers the overflow protection of Money, we would essentially
    // be losing money as taking and repaying the loan immediately would not get us back to
    // the same bank balance anymore.
    if read_field!(c, money) > i64::MAX.saturating_sub(loan) {
        crate::command_func::cmd_error(out);
        return;
    }

    if flags.test(DoCommandFlag::Execute) {
        write_field!(c, money, read_field!(c, money).saturating_add(loan));
        write_field!(
            c,
            current_loan,
            read_field!(c, current_loan).saturating_add(loan)
        );
        (s.finance.invalidate_company_windows)(current.0);
    }

    (s.cost.expense_money)(out.0, ExpensesType::EXPENSES_OTHER.0, 0);
}

/// `CmdDecreaseLoan`: decrease the loan of your company.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn cmd_decrease_loan(s: &Services, out: Cost<'_>, flags: DoCommandFlags, args: CmdLoan) {
    let CmdLoan { cmd, amount } = args;
    let current = CompanyID((s.shared.current_company)());
    let c = CompanyRef::get(s.identity, current).finances;

    if read_field!(c, current_loan) == 0 {
        out.error(STR_ERROR_LOAN_ALREADY_REPAID.0);
        return;
    }

    let interval = i64::from(LOAN_INTERVAL);
    let loan = match cmd {
        // Pay back one step
        LoanCommand::INTERVAL => read_field!(c, current_loan).min(interval),
        // Pay back as much as possible
        LoanCommand::MAX => {
            let mut loan = read_field!(c, current_loan)
                .min(get_available_money(s.shared, s.finance, current))
                .max(interval);
            loan = loan.saturating_sub(loan % interval);
            loan
        }
        // Repay the given amount of loan
        LoanCommand::AMOUNT => {
            let loan = amount;
            if loan % interval != 0 || loan < interval || loan > read_field!(c, current_loan) {
                // Invalid amount to loan
                crate::command_func::cmd_error(out);
                return;
            }
            loan
        }
        // Invalid method
        _ => {
            crate::command_func::cmd_error(out);
            return;
        }
    };

    if get_available_money(s.shared, s.finance, current) < loan {
        (s.cost.error_param)(out.0, STR_ERROR_CURRENCY_REQUIRED.0, loan.cast_unsigned());
        return;
    }

    if flags.test(DoCommandFlag::Execute) {
        write_field!(c, money, read_field!(c, money).saturating_sub(loan));
        write_field!(
            c,
            current_loan,
            read_field!(c, current_loan).saturating_sub(loan)
        );
        (s.finance.invalidate_company_windows)(current.0);
    }
    out.zero();
}

/// `CmdIncreaseLoan`, for the native facade.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_increase_loan(
    s: &Services,
    out: *mut CommandCost,
    flags: u16,
    cmd: u8,
    amount: i64,
) {
    cmd_increase_loan(
        s,
        Cost(out, s.cost),
        DoCommandFlags(flags),
        CmdLoan {
            cmd: LoanCommand(cmd),
            amount,
        },
    );
}

/// `CmdDecreaseLoan`, for the native facade.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_decrease_loan(
    s: &Services,
    out: *mut CommandCost,
    flags: u16,
    cmd: u8,
    amount: i64,
) {
    cmd_decrease_loan(
        s,
        Cost(out, s.cost),
        DoCommandFlags(flags),
        CmdLoan {
            cmd: LoanCommand(cmd),
            amount,
        },
    );
}

/// Arguments of `CmdSetCompanyMaxLoan`, in command order.
#[derive(Clone, Copy)]
pub(crate) struct CmdSetCompanyMaxLoan {
    /// The company ID.
    pub company: CompanyID,
    /// The new max loan amount, will be rounded down to the multitude of `LOAN_INTERVAL`. If
    /// set to `COMPANY_MAX_LOAN_DEFAULT` reset the max loan to default(global) value.
    pub amount: i64,
}

/// `CmdSetCompanyMaxLoan`: sets the max loan amount of your company. Does not respect the
/// global loan setting.
#[allow(unsafe_code)] // Field-sized access to live native records, under the module contract.
pub(crate) fn cmd_set_company_max_loan(
    s: &Services,
    out: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdSetCompanyMaxLoan,
) {
    let CmdSetCompanyMaxLoan {
        company,
        mut amount,
    } = args;
    if (s.shared.current_company)() != Owner::DEITY.0 {
        crate::command_func::cmd_error(out);
        return;
    }
    if amount != COMPANY_MAX_LOAN_DEFAULT && !(0..=MAX_LOAN_LIMIT).contains(&amount) {
        crate::command_func::cmd_error(out);
        return;
    }

    let c = (s.finance.company_finances)(company.0);
    if c.is_null() {
        crate::command_func::cmd_error(out);
        return;
    }

    if flags.test(DoCommandFlag::Execute) {
        // Round the amount down to a multiple of LOAN_INTERVAL.
        if amount != COMPANY_MAX_LOAN_DEFAULT {
            amount -= amount % i64::from(LOAN_INTERVAL);
        }

        write_field!(c, max_loan, amount);
        (s.finance.invalidate_company_windows)(company.0);
    }
    out.zero();
}

/// `CmdSetCompanyMaxLoan`, for the native facade.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_set_max_loan(
    s: &Services,
    out: *mut CommandCost,
    flags: u16,
    company: u8,
    amount: i64,
) {
    cmd_set_company_max_loan(
        s,
        Cost(out, s.cost),
        DoCommandFlags(flags),
        CmdSetCompanyMaxLoan {
            company: CompanyID(company),
            amount,
        },
    );
}

/// Arguments of `CmdChangeBankBalance`, in command order.
#[derive(Clone, Copy)]
pub(crate) struct CmdChangeBankBalance {
    /// Tile to show text effect on (if not 0).
    pub tile: TileIndex,
    /// The amount of money to receive (if positive), or spend (if negative).
    pub delta: i64,
    /// The company ID.
    pub company: CompanyID,
    /// The expenses type which should register the cost/income.
    pub expenses_type: ExpensesType,
}

/// `CmdChangeBankBalance`: change the bank bank balance of a company by inserting or removing
/// money without affecting the loan.
pub(crate) fn cmd_change_bank_balance(
    s: &Services,
    out: Cost<'_>,
    flags: DoCommandFlags,
    args: CmdChangeBankBalance,
) {
    let CmdChangeBankBalance {
        tile,
        delta,
        company,
        expenses_type,
    } = args;
    if !(s.shared.company_valid)(company.0) {
        crate::command_func::cmd_error(out);
        return;
    }
    if expenses_type >= ExpensesType::EXPENSES_END {
        crate::command_func::cmd_error(out);
        return;
    }
    let current = (s.shared.current_company)();
    if current != Owner::DEITY.0 {
        crate::command_func::cmd_error(out);
        return;
    }

    if flags.test(DoCommandFlag::Execute) {
        // Change company bank balance of company.
        (s.finance.set_current_company)(company.0);
        subtract_money_from_company(s, neg(delta), expenses_type);
        (s.finance.set_current_company)(current);

        if tile.0 != 0 {
            let world = crate::map_func::World::new(s.shared);
            let map = world.dimensions;
            // `GetTilePixelZ(tile)` is `GetTileZ(tile) * TILE_HEIGHT`.
            (s.shared.cost_animation)(
                (map.tile_x(tile) * TILE_SIZE).cast_signed(),
                (map.tile_y(tile) * TILE_SIZE).cast_signed(),
                world.tile_z(tile) * TILE_HEIGHT.cast_signed(),
                neg(delta),
            );
        }
    }

    // This command doesn't cost anything for deity.
    (s.cost.expense_money)(out.0, expenses_type.0, 0);
}

/// `CmdChangeBankBalance`, for the native facade.
#[unsafe(no_mangle)]
#[allow(unsafe_code)] // The native export and its conversion of the native arguments.
pub extern "C" fn openttd_rust_company_change_bank_balance(
    s: &Services,
    out: *mut CommandCost,
    flags: u16,
    tile: u32,
    delta: i64,
    company: u8,
    expenses_type: u8,
) {
    cmd_change_bank_balance(
        s,
        Cost(out, s.cost),
        DoCommandFlags(flags),
        CmdChangeBankBalance {
            tile: TileIndex(tile),
            delta,
            company: CompanyID(company),
            expenses_type: ExpensesType(expenses_type),
        },
    );
}

crate::abi::layouts! {
    fn abi_layout;
    "CompanyEconomyEntry" ["company_economy"] => CompanyEconomyEntry {
        income,
        expenses,
        delivered_cargo,
        performance_history,
        company_value,
    };
    "CompanyFinances" ["company_economy"] => Finances {
        money,
        money_fraction,
        current_loan,
        max_loan,
        block_preview,
        months_empty,
        months_of_bankruptcy,
        bankrupt_asked,
        bankrupt_timeout,
        bankrupt_value,
        terraform_limit,
        clear_limit,
        tree_limit,
        build_object_limit,
        yearly_expenses,
        cur_economy,
        old_economy,
        num_valid_stat_ent,
    };
    "Economy" ["company_economy"] => Economy {
        max_loan,
        fluct,
        interest_rate,
        infl_amount,
        infl_amount_pr,
        inflation_prices,
        inflation_payment,
        old_max_loan_unround,
        old_max_loan_unround_fract,
    };
    "OpenTTDCompanyFinanceServices" ["company_economy"] => FinanceServices {
        current_company,
        set_current_company,
        networking,
        company_finances,
        company_get_finances,
        invalidate_company_windows,
    };
    "OpenTTDCompanyServices" ["company_economy"] => Services {
        cost,
        finance,
        identity,
        shared,
        engines,
        objectives,
        settings,
        station_lifecycle,
        subsidies,
        ai_can_start_new,
        ai_stop,
        allocate_company,
        allow_list_add,
        allow_list_remove,
        can_allocate_company,
        canal_maintenance_cost,
        cargo_initial_payment,
        cargo_set_current_payment,
        change_window_owner,
        close_company_windows,
        close_construction_windows,
        company_admin_remove,
        company_count,
        delete_company,
        delete_goal,
        delete_story_page,
        draw_string_company_colour,
        group_all,
        location_of_hq,
        network_admin_company_new,
        network_clients_to_spectators,
        network_server,
        new_competitor_timeout_has_fired,
        new_competitor_timeout_reset,
        next_goal,
        next_sign,
        next_story_page,
        next_waypoint,
        parse_company_manager_face,
        price_base_category,
        price_base_start_price,
        rail_maintenance_cost,
        road_maintenance_cost,
        script_random_next,
        send_company_manager_face,
        set_avail_railtypes,
        set_avail_roadtypes,
        set_default_company_settings,
        set_inaugurated_year,
        set_inaugurated_year_calendar,
        set_unitnumber,
        show_buy_company_dialog,
        show_company_finances,
        sign_owner,
        signal_maintenance_cost,
        snd_play_fx,
        station_maintenance_cost,
        sync_company_settings,
        use_free_group,
        use_free_unit,
    };
    "OpenTTDCompanyStartup" ["company_economy"] => Startup {
        is_ai,
        company,
        stage,
        index,
        created,
    };
    "OpenTTDCompanyControl" ["company_economy"] => Control {
        args,
        flags,
        stage,
        startup,
    };
}

/// Test access to `_price`.
#[cfg(test)]
pub(crate) mod testing {
    use crate::economy_type::Price;

    /// `_price[index] = value`, for a test that holds the game-state test lock.
    pub(crate) fn set_price(index: Price, value: i64) {
        super::set_price(index, value);
    }
}

#[cfg(test)]
#[allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]
mod tests;
