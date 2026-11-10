/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */
//! Company finance, history and economy lifecycle. Reentrant services (company deletion
//! `Post`, tile owner changes, pool deletion, nested commands, `StopAI`) run directly after
//! all raw field accesses end. `AI::StartNew` and the `CCA_NEW_AI` `Post` can raise
//! `Script_FatalError`, so they return to C++ through a caller-owned stack frame.
#![allow(
    unsafe_code,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::many_single_char_names,
    clippy::struct_field_names,
    clippy::struct_excessive_bools,
    clippy::fn_params_excessive_bools,
    clippy::verbose_bit_mask
)]
use crate::company_type::{CompanyID, INVALID_OWNER, MAX_COMPANIES, OWNER_DEITY, OWNER_NONE};
use crate::economy_type::EXPENSES_OTHER;
use crate::raw_field::{read_field, write_field};
use crate::tile_owner::change_tile_owner;
use crate::tile_type::TileIndex;
use std::ptr::{addr_of, addr_of_mut};
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Entry {
    pub income: i64,
    pub expenses: i64,
    pub cargo: [u32; 64],
    pub performance: i32,
    pub value: i64,
}
#[repr(C)]
pub struct Finances {
    pub money: i64,
    pub fraction: u8,
    pub loan: i64,
    pub max_loan: i64,
    pub preview: u8,
    pub empty: u8,
    pub bankruptcy: u8,
    pub asked: u16,
    pub timeout: i16,
    pub bankrupt_value: i64,
    pub terraform: u32,
    pub clear: u32,
    pub tree: u32,
    pub object: u32,
    pub expenses: [[i64; 13]; 3],
    pub current: Entry,
    pub old: [Entry; 24],
    pub valid: u8,
}
#[repr(C)]
pub struct Economy {
    pub max_loan: i64,
    pub fluct: i16,
    pub interest: u8,
    pub infl_amount: u8,
    pub infl_payment: u8,
    pub prices: u64,
    pub payment: u64,
    pub old_loan: i64,
    pub old_fraction: u16,
}
/// SAFETY: Live serialized company finance owner and valid cargo slot.
pub(crate) unsafe fn record_delivery(owner: *mut Finances, cargo: u8, amount: u32) {
    unsafe {
        let count = &mut (*owner).current.cargo[usize::from(cargo)];
        *count = count.wrapping_add(amount);
    }
}
/// Opaque native `Company`. Pool services lend its address while the shell is
/// live; Rust never dereferences it or retains an owner borrow across callbacks.
pub use crate::company_type::OpenTTDCompany;

/// A live pool company and its canonical finance storage; `id == u32::MAX` ends.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CompanyRef {
    pub finances: *mut Finances,
    pub id: u32,
}
/// `GroupStatistics::num_vehicle` per buildable type, or the per-company limits.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VehicleCounts {
    pub trains: u16,
    pub road: u16,
    pub ships: u16,
    pub aircraft: u16,
}
/// `CompanyInfrastructure` counts read once per maintenance payment.
#[repr(C)]
pub struct Infrastructure {
    pub rail: [u32; 64],
    pub road: [u32; 63],
    pub rail_total: u32,
    pub road_total: u32,
    pub tram_total: u32,
    pub signal: u32,
    pub water: u32,
    pub station: u32,
    /// Bit `rt` set when `RoadTypeIsRoad(rt)`.
    pub road_is_road: u64,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VehicleGroupFlags {
    pub engine_countable: bool,
    pub primary: bool,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VehicleProfit {
    pub profit_last_year: i64,
    pub economy_age: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct StationTimes {
    pub since_load: u8,
    pub since_unload: u8,
}
/// Town fields read for the old owner; `rating` is `ratings[old]`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TownRights {
    pub rating: i16,
    pub have_ratings: u16,
    pub exclusive_counter: u8,
    pub exclusivity: u8,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PriceBase {
    pub start_price: i64,
    pub category: u8,
}
/// Command result: `error` 0 success, 1 `CMD_ERROR`, 2 maximum loan (`param`),
/// 3 loan repaid, 4 currency required (`param`), 5 insufficient funds,
/// 6 too many vehicles. `expense` 255 is `INVALID_EXPENSES`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Cost {
    pub cost: i64,
    pub param: i64,
    pub error: u8,
    pub expense: u8,
}
/// Caller-owned `DoStartupNewCompany` frame; `true` requests `AI::StartNew(company)`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Startup {
    pub company: u32,
    pub requested: u8,
    pub is_ai: bool,
    pub stage: u8,
}
/// Caller-owned `CmdCompanyCtrl` frame; `true` requests `AI::StartNew(startup.company)`.
#[repr(C)]
pub struct Control {
    pub result: Cost,
    pub startup: Startup,
    pub client: u32,
    pub action: u8,
    pub target: u8,
    pub reason: u8,
    pub execute: bool,
    pub stage: u8,
}
/// Continuation locals across the native `CCA_NEW_AI` Post; settings are read live.
#[derive(Default)]
pub(crate) struct Tick {
    num_companies: usize,
    index: i32,
    timeout: i32,
    num_ais: u8,
    stage: u8,
}
/// Settings of the economy-month entry.
#[repr(C)]
pub struct Month {
    pub month: u8,
    pub infinite_money: bool,
    pub maintenance: bool,
    pub fluctuating: bool,
}
#[repr(C)]
pub struct Year {
    pub local_finances: *mut Finances,
    pub local: u8,
    pub show_finances: bool,
    pub new_year_sound: bool,
}
/// Calendar year and difficulty used by inflation, prices and startup.
#[repr(C)]
pub struct EconomySettings {
    pub year: i32,
    pub max_loan: u32,
    pub initial_interest: u8,
    pub vehicle_costs: u8,
    pub construction_cost: u8,
    pub inflation: bool,
}
/// Finance services used by hot money paths and financial commands.
#[repr(C)]
pub struct FinanceServices {
    pub current_company: extern "C" fn() -> u8,
    pub set_current_company: extern "C" fn(u8),
    pub networking: extern "C" fn() -> bool,
    pub company_finances: extern "C" fn(u8) -> *mut Finances,
    pub invalidate_company_windows: extern "C" fn(u8),
    pub give_money_message: extern "C" fn(u8, i64),
    pub money_animation: extern "C" fn(u32, i64),
}
/// Lifecycle, economy and ownership-transfer services; see `company_ffi.h`.
#[repr(C)]
pub struct Services {
    pub identity: &'static crate::company_identity::Services,
    pub finance: &'static FinanceServices,
    pub shared: &'static crate::services::Services,
    pub ai_can_start: extern "C" fn() -> bool,
    pub allow_list_add:
        extern "C" fn(crate::company_type::CompanyID, crate::services::ByteSpan) -> bool,
    pub allow_list_remove:
        extern "C" fn(crate::company_type::CompanyID, crate::services::ByteSpan) -> bool,
    pub assign_company_context: extern "C" fn(crate::company_type::CompanyID),
    pub close_construction_windows: extern "C" fn(),
    pub cost: &'static crate::services::CommandCostServices,
    pub objectives: extern "C" fn() -> &'static crate::objectives::Services,
    pub owned_by: u32,
    pub settings: extern "C" fn() -> &'static crate::settings_table::Services,
    pub company_next: extern "C" fn(u32) -> CompanyRef,
    pub company_count: extern "C" fn() -> usize,
    pub competitor_fired: extern "C" fn() -> bool,
    pub can_allocate_company: extern "C" fn() -> bool,
    pub local_company: extern "C" fn() -> u8,
    pub network_server: extern "C" fn() -> bool,
    pub company_vehicle_counts: extern "C" fn(u8) -> VehicleCounts,
    pub vehicle_limits: extern "C" fn() -> VehicleCounts,
    pub company_in_trouble: extern "C" fn(u8),
    pub hq_location: extern "C" fn(u8) -> u32,
    pub object_services: extern "C" fn() -> *const crate::objects::ObjectServices,
    pub performance_detail_dirty: extern "C" fn(),
    pub company_graphs_dirty: extern "C" fn(),
    pub company_infrastructure: extern "C" fn(u8, *mut Infrastructure),
    pub rail_maintenance_cost: extern "C" fn(u8, u32, u32) -> i64,
    pub signal_maintenance_cost: extern "C" fn(u32) -> i64,
    pub road_maintenance_cost: extern "C" fn(u8, u32, u32) -> i64,
    pub canal_maintenance_cost: extern "C" fn(u32) -> i64,
    pub station_maintenance_cost: extern "C" fn(u32) -> i64,
    pub station_lifecycle: extern "C" fn() -> *const crate::station_lifecycle::Services,
    pub recession_news: extern "C" fn(bool),
    pub price_base: extern "C" fn(u32) -> PriceBase,
    pub cargo_next: extern "C" fn(u32, *mut i64) -> u32,
    pub set_cargo_payment: extern "C" fn(u32, i64),
    pub price_windows_dirty: extern "C" fn(),
    pub industry_daily_changes: extern "C" fn(bool),
    pub ask_merger: extern "C" fn(u8, u8, i64),
    pub is_interactive_company: extern "C" fn(u8) -> bool,
    pub show_buy_company: extern "C" fn(u8),
    pub script_random_next: extern "C" fn(u32) -> u32,
    pub reset_competitor_timeout: extern "C" fn(u32),
    pub finances_dirty: extern "C" fn(u8),
    pub show_company_finances: extern "C" fn(u8),
    pub new_year_sound: extern "C" fn(bool),
    pub allocate_company: extern "C" fn(u8, bool) -> CompanyRef,
    pub prepare_new_company: extern "C" fn(u8, bool) -> bool,
    pub company_defaults: extern "C" fn(u8),
    pub startup_windows: extern "C" fn(),
    pub new_company_events: extern "C" fn(u8),
    pub company_league_dirty: extern "C" fn(),
    pub company_ctrl_windows: extern "C" fn(),
    pub close_network_status: extern "C" fn(),
    pub network_spectate: extern "C" fn(u32),
    pub admin_company_new: extern "C" fn(u8),
    pub network_own_company: extern "C" fn(u8, u32) -> *mut crate::company_type::OpenTTDCompany,
    pub network_own_company_settings: extern "C" fn(*mut crate::company_type::OpenTTDCompany),
    pub assert_new_ai_slot: extern "C" fn(u8),
    pub company_bankrupt_news: extern "C" fn(u8),
    pub stop_ai: extern "C" fn(u8),
    pub delete_company: extern "C" fn(u8),
    pub company_removed: extern "C" fn(u8, u8),
    pub merger_news: extern "C" fn(u8, bool),
    pub acquisition_windows: extern "C" fn(u8),
    pub clients_to_spectators: extern "C" fn(u8),
    pub subsidies: extern "C" fn() -> *const crate::subsidy::Services,
    pub town_next: extern "C" fn(u32, u8, *mut TownRights) -> u32,
    pub town_rating: extern "C" fn(u32, u8) -> i16,
    pub set_town_rating: extern "C" fn(u32, u8, i16, bool),
    pub set_town_exclusivity: extern "C" fn(u32, u8, u8),
    pub vehicle_next_owned: extern "C" fn(u8, u32, *mut u8) -> u32,
    pub vehicle_value: extern "C" fn(u32) -> i64,
    pub vehicle_is_primary: extern "C" fn(u32) -> bool,
    pub vehicle_profit: extern "C" fn(u32) -> VehicleProfit,
    pub vehicle_previous: extern "C" fn(u32) -> u32,
    pub vehicle_group_flags: extern "C" fn(u32) -> VehicleGroupFlags,
    pub vehicle_service_interval_is_custom: extern "C" fn(u32) -> bool,
    pub delete_vehicle: extern "C" fn(u32),
    pub count_group_engine: extern "C" fn(u32, i32),
    pub count_group_vehicle: extern "C" fn(u32, i32),
    pub reset_service_interval: extern "C" fn(u32, i32, bool),
    pub set_vehicle_owner: extern "C" fn(u32, u8),
    pub assign_unit_number: extern "C" fn(u32, u8),
    pub remove_engine_replacements: extern "C" fn(u8),
    pub group_next_owned: extern "C" fn(u8, u32) -> u32,
    pub delete_group: extern "C" fn(u32),
    pub transfer_group: extern "C" fn(u32, u8),
    pub copy_service_interval_defaults: extern "C" fn(u8, u8),
    pub update_autoreplace: extern "C" fn(u8),
    pub add_track_to_signal_buffer: extern "C" fn(u32, u8, u8),
    pub update_signals_in_buffer: extern "C" fn(),
    pub add_airport_infrastructure: extern "C" fn(u8, u8),
    pub station_next_owned: extern "C" fn(u8, u32) -> u32,
    pub station_times: extern "C" fn(u32) -> StationTimes,
    pub set_station_owner: extern "C" fn(u32, u8),
    pub waypoint_next_owned: extern "C" fn(u8, u32) -> u32,
    pub set_waypoint_owner: extern "C" fn(u32, u8),
    pub sign_next_owned: extern "C" fn(u8, u32) -> u32,
    pub set_sign_owner: extern "C" fn(u32, u8),
    pub goal_next_owned: extern "C" fn(u8, u32) -> u32,
    pub delete_goal: extern "C" fn(u32),
    pub story_page_next_owned: extern "C" fn(u8, u32) -> u32,
    pub delete_story_page: extern "C" fn(u32),
    pub change_window_owner: extern "C" fn(u8, u8),
}
/// `SetLocalCompany`: team chat observes the old context before the assertion and assignment.
pub(crate) fn set_local_company(s: &Services, company: crate::company_type::CompanyID) {
    use crate::company_type::CompanyID;
    use crate::window_type::{
        WC_COMPANY, WC_COMPANY_COLOUR, WC_GOALS_LIST, WC_SEND_NETWORK_MSG, WC_SIGN_LIST,
    };
    crate::movement::native_assert(
        s.shared,
        (s.shared.company_valid)(company.0)
            || company == CompanyID::SPECTATOR
            || company == CompanyID::NONE,
    );
    let switching = (s.local_company)() != company.0;
    let windows = (s.objectives)();
    if switching {
        (windows.window_data)(
            WC_SEND_NETWORK_MSG,
            crate::network_type::DESTTYPE_TEAM,
            i32::from((s.local_company)()),
        );
    }
    crate::movement::native_assert(
        s.shared,
        (s.local_company)() == (s.finance.current_company)(),
    );
    (s.assign_company_context)(company);
    if switching {
        (windows.window_classes)(WC_COMPANY, 0);
        (s.close_construction_windows)();
    }
    (s.identity.shared.screen_dirty)();
    (windows.window_classes)(WC_SIGN_LIST, -1);
    (windows.window_classes)(WC_GOALS_LIST, 0);
    (windows.window_classes)(WC_COMPANY_COLOUR, -1);
    (s.identity.reset_colourmap)();
}

pub(crate) fn check_ownership(
    s: &Services,
    out: crate::services::Cost<'_>,
    owner: crate::company_type::Owner,
    tile: crate::tile_type::TileIndex,
) {
    use crate::company_type::{OWNER_END, OWNER_TOWN};
    crate::movement::native_assert(s.shared, owner.0 < OWNER_END);
    crate::movement::native_assert(s.shared, owner.0 != OWNER_TOWN || tile.0 != 0);
    if owner.0 == (s.finance.current_company)() {
        out.zero();
        return;
    }
    out.error(s.owned_by);
    if (s.local_company)() == (s.finance.current_company)() {
        (out.1.encode_owned_by)(out.0, owner, tile);
        if owner.0 != OWNER_TOWN {
            (out.1.error_owner)(out.0, owner);
        }
    }
}

pub(crate) fn check_tile_ownership(
    s: &Services,
    out: crate::services::Cost<'_>,
    tile: crate::tile_type::TileIndex,
) {
    let owner = crate::company_type::CompanyID(crate::map_func::World::new(s.shared).owner(tile.0));
    check_ownership(s, out, owner, tile);
}

fn execute_allow_list_action(
    s: &Services,
    action: u8,
    company: crate::company_type::CompanyID,
    key: crate::services::ByteSpan,
) -> bool {
    use crate::company_type::{CALCA_ADD, CALCA_REMOVE};
    match action {
        CALCA_ADD => (s.allow_list_add)(company, key),
        CALCA_REMOVE => (s.allow_list_remove)(company, key),
        _ => {
            (s.shared.not_reached)();
            unreachable!()
        }
    }
}

fn allow_list(
    s: &Services,
    out: crate::services::Cost<'_>,
    flags: u16,
    action: u8,
    key: crate::services::ByteSpan,
) {
    use crate::company_type::{CALCA_ADD, CALCA_REMOVE, CompanyID};
    let company = CompanyID((s.finance.current_company)());
    if !(s.shared.company_valid)(company.0)
        || key.size != crate::network_type::NETWORK_PUBLIC_KEY_LENGTH - 1
        || !matches!(action, CALCA_ADD | CALCA_REMOVE)
    {
        out.error(crate::strings_type::INVALID_STRING_ID);
        return;
    }
    if flags & crate::command_type::DoCommandFlag::EXECUTE != 0
        && execute_allow_list_action(s, action, company, key)
    {
        ((s.objectives)().window_data)(crate::window_type::WC_CLIENT_LIST, 0, 0);
        let transactions = (s.settings)().refit.common.transactions;
        ((transactions.build_services)().rail.company_dirty)();
    }
    out.zero();
}

/// Read only the selected field of the supplied company's defaults, or the client defaults for null.
pub(crate) fn service_interval(
    settings: &crate::settings_table::Services,
    company: Option<crate::company_type::CompanyID>,
    kind: crate::vehicle_type::VehicleType,
) -> i32 {
    let defaults = match company {
        None => (settings.client_vehicle_defaults)(),
        Some(company) => (settings.company_vehicle_defaults)(company),
    };
    service_interval_from(settings.shared, defaults, kind)
}

fn service_interval_from(
    shared: &crate::services::Services,
    defaults: *const crate::settings_table::VehicleDefaults,
    kind: crate::vehicle_type::VehicleType,
) -> i32 {
    use crate::vehicle_type::{VEH_AIRCRAFT, VEH_ROAD, VEH_SHIP, VEH_TRAIN};
    i32::from(match kind.0 {
        VEH_TRAIN => read_field!(defaults, servint_trains),
        VEH_ROAD => read_field!(defaults, servint_roadveh),
        VEH_AIRCRAFT => read_field!(defaults, servint_aircraft),
        VEH_SHIP => read_field!(defaults, servint_ships),
        _ => {
            (shared.not_reached)();
            unreachable!()
        }
    })
}

pub(crate) fn first_playable(s: &Services) -> crate::company_type::CompanyID {
    use crate::company_type::CompanyID;
    let mut from = 0;
    loop {
        let c = (s.identity.next_company)(from);
        if c.is_null() {
            break;
        }
        let id = (s.identity.company_id)(c);
        // IsHumanID resolves through Company::Get at this point in the original.
        if !(s.identity.is_ai)((s.identity.company)(id)) {
            return id;
        }
        from = u32::from(id.0) + 1;
    }
    if (s.can_allocate_company)() {
        for id in CompanyID::BEGIN.0..MAX_COMPANIES {
            if !(s.shared.company_valid)(id) {
                return CompanyID(id);
            }
        }
    }
    CompanyID::BEGIN
}

#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_set_local(
    s: &Services,
    company: crate::company_type::CompanyID,
) {
    set_local_company(s, company);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_check_ownership(
    s: &Services,
    out: *mut crate::services::CommandCost,
    owner: crate::company_type::Owner,
    tile: crate::tile_type::TileIndex,
) {
    check_ownership(s, crate::services::Cost(out, s.cost), owner, tile);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_check_tile_ownership(
    s: &Services,
    out: *mut crate::services::CommandCost,
    tile: crate::tile_type::TileIndex,
) {
    check_tile_ownership(s, crate::services::Cost(out, s.cost), tile);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_allow_list(
    s: &Services,
    out: *mut crate::services::CommandCost,
    flags: u16,
    action: u8,
    key: crate::services::ByteSpan,
) {
    allow_list(s, crate::services::Cost(out, s.cost), flags, action, key);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_service_interval(
    s: &Services,
    company: *mut crate::company_type::OpenTTDCompany,
    kind: crate::vehicle_type::VehicleType,
) -> i32 {
    service_interval(
        (s.settings)(),
        if company.is_null() {
            None
        } else {
            Some((s.identity.company_id)(company))
        },
        kind,
    )
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_first_playable(
    s: &Services,
) -> crate::company_type::CompanyID {
    first_playable(s)
}

static mut ECONOMY: Economy = unsafe { std::mem::zeroed() };
static mut PRICES: [i64; 71] = [0; 71];
static mut SCORES: [[i64; 10]; 15] = [[0; 10]; 15];
static mut MULTIPLIERS: [i8; 71] = [0; 71];
static mut TICK: u32 = 0;
// SAFETY: The caller resolves a live canonical allocation on the serial game
// thread. Field types/layout are native-ABI checked. Each read/write finishes
// before a shared service; no reference is created or retained.
// Stable Rust allocations also carry C++ view lifetimes. Access is raw and serial.
/// Allocate zeroed private finance storage; C++ immediately starts its field objects.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_state_create() -> *mut Finances {
    Box::into_raw(Box::new(unsafe { std::mem::zeroed() }))
}
/// Release the sole allocation after its C++ field objects and all accesses end.
/// # Safety
/// Pointer is a live create result, owned exclusively by the caller, destroyed once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_company_state_destroy(p: *mut Finances) {
    unsafe { drop(Box::from_raw(p)) };
}
/// Stable process-lifetime economy storage; C++ constructs its scalar view once.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_economy_state() -> *mut Economy {
    &raw mut ECONOMY
}
/// Current `_price[index]` for other Rust owners under the game-state mutex.
pub(crate) fn price(index: usize) -> i64 {
    // SAFETY: process-lifetime storage; game-state mutex serializes access and
    // callers pass a source `Price` index below `PR_END`.
    unsafe { (&raw const PRICES).cast::<i64>().add(index).read() }
}
/// Stable process-lifetime prices; no concurrent accesses are permitted.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_economy_prices() -> *mut i64 {
    (&raw mut PRICES).cast()
}
/// Stable score matrix with original typed-company indexing.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_scores() -> *mut i64 {
    (&raw mut SCORES).cast()
}
/// Stable legacy/DATE tick-cursor address.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_tick() -> *mut u32 {
    &raw mut TICK
}
const INVALID: u32 = u32::MAX;
const PRICE_COUNT: usize = 71;
const INITIAL_LOAN: i64 = 100_000;
const LOAN_INTERVAL: i64 = 10_000;
const MAX_INFLATION: u64 = (1_u64 << 31) - 1;
const MAX_LOAN_LIMIT: i64 = 2_000_000_000;
use crate::timer_game_common::{ORIGINAL_BASE_YEAR, ORIGINAL_MAX_YEAR};
use crate::timer_game_tick::{DAY_TICKS, TICKS_PER_SECOND};
/// `VEHICLE_PROFIT_MIN_AGE` in economy days.
const PROFIT_MIN_AGE: i32 = 730;
const EXPENSES_PROPERTY: usize = 6;
const EXPENSES_LOAN_INTEREST: usize = 11;
const EXPENSES_END: u8 = 13;
const NO_EXPENSE: u8 = 255;
const SCORE_NEEDED: [i64; 9] = [
    120, 80, 10_000, 50_000, 100_000, 40_000, 8, 10_000_000, 250_000,
];
const SCORE_WEIGHT: [i64; 9] = [100, 100, 100, 50, 100, 400, 50, 50, 50];
fn e() -> *mut Economy {
    &raw mut ECONOMY
}
/// Saturating `-Money` (`OverflowSafeInt64::operator-`).
pub(crate) fn neg(a: i64) -> i64 {
    if a == i64::MIN { i64::MAX } else { -a }
}
fn done(cost: i64, expense: u8) -> Cost {
    Cost {
        cost,
        param: 0,
        error: 0,
        expense,
    }
}
fn failed(error: u8, param: i64) -> Cost {
    Cost {
        cost: 0,
        param,
        error,
        expense: NO_EXPENSE,
    }
}
fn error() -> Cost {
    failed(1, 0)
}
/// Visit a pool in source order, resuming after the visited index so deletion
/// inside `body` behaves like the original pool iterator.
fn each(next: impl Fn(u32) -> u32, mut body: impl FnMut(u32)) {
    let mut from = 0_u32;
    loop {
        let id = next(from);
        if id == INVALID {
            return;
        }
        from = id.wrapping_add(1);
        body(id);
    }
}
fn companies(s: &Services, mut body: impl FnMut(*mut Finances, u8)) {
    let mut from = 0_u32;
    loop {
        let c = (s.company_next)(from);
        if c.id == INVALID {
            return;
        }
        from = c.id.wrapping_add(1);
        body(c.finances, c.id as u8);
    }
}
fn maxloan(p: *const Finances) -> i64 {
    let max = read_field!(p, max_loan);
    if max == i64::MIN {
        read_field!(e(), max_loan)
    } else {
        max
    }
}
fn available(p: *const Finances, infinite: bool) -> i64 {
    if infinite || p.is_null() {
        i64::MAX
    } else {
        read_field!(p, money)
    }
}
fn subtract(f: &FinanceServices, p: *mut Finances, id: u8, cost: i64, expense: usize) {
    if cost == 0 {
        return;
    }
    write_field!(p, money, read_field!(p, money).saturating_sub(cost));
    write_field!(
        p,
        expenses[0][expense],
        read_field!(p, expenses[0][expense]).saturating_add(cost)
    );
    if (7..=10).contains(&expense) {
        write_field!(
            p,
            current.income,
            read_field!(p, current.income).saturating_sub(cost)
        );
    } else if (2..=6).contains(&expense) || expense == 11 {
        write_field!(
            p,
            current.expenses,
            read_field!(p, current.expenses).saturating_sub(cost)
        );
    }
    (f.invalidate_company_windows)(id);
}
fn subtract_fraction(f: &FinanceServices, p: *mut Finances, id: u8, mut cost: i64, expense: usize) {
    let old = read_field!(p, fraction);
    let next = old.wrapping_sub(cost as u8);
    write_field!(p, fraction, next);
    cost >>= 8;
    if next > old {
        cost = cost.saturating_add(1);
    }
    if cost != 0 {
        subtract(f, p, id, cost, expense);
    }
}
/// The `StationID` of a station pool index that `NextOwned<Station>` yields.
#[allow(
    clippy::cast_possible_truncation,
    reason = "StationID is the original uint16_t station pool index"
)]
fn station_id(index: u32) -> crate::station_type::StationID {
    crate::station_type::StationID(index as u16)
}
/// `Station::Get(id)->facilities.Count()`, through the shared facility reader.
fn station_facility_count(s: &Services, id: crate::station_type::StationID) -> u32 {
    (s.shared.station_facilities)(id.0).count_ones()
}
/// `Aircraft::From(Vehicle::Get(id))->IsNormalAircraft()`. For a valid ID, the first
/// vehicle at or after it is that vehicle.
fn aircraft_is_normal(s: &Services, id: crate::vehicle_type::VehicleID) -> bool {
    (s.shared.vehicle_next)(id.0).is_some_and(|v| crate::aircraft::is_normal_aircraft(v.subtype()))
}
fn assets(s: &Services, id: u8) -> i64 {
    let mut num = 0_u32;
    each(
        |from| (s.station_next_owned)(id, from),
        |st| num = num.wrapping_add(station_facility_count(s, station_id(st))),
    );
    let price = unsafe { (&raw const PRICES).cast::<i64>().read() };
    let mut value = price.saturating_mul(i64::from(num)).saturating_mul(25);
    let mut kind = 0_u8;
    let mut from = 0_u32;
    loop {
        let v = (s.vehicle_next_owned)(id, from, &raw mut kind);
        if v == INVALID {
            break;
        }
        from = v.wrapping_add(1);
        if kind == 0
            || kind == 1
            || (kind == 3 && aircraft_is_normal(s, crate::vehicle_type::VehicleID(v)))
            || kind == 2
        {
            value = value.saturating_add((s.vehicle_value)(v).saturating_mul(3) >> 1);
        }
    }
    value
}
fn value(s: &Services, p: *mut Finances, id: u8, including_loan: bool) -> i64 {
    let mut amount = assets(s, id);
    if including_loan {
        amount = amount.saturating_sub(read_field!(p, loan));
    }
    amount.saturating_add(read_field!(p, money)).max(1)
}
fn hostile_value(s: &Services, p: *mut Finances, id: u8) -> i64 {
    let mut amount = assets(s, id);
    amount = amount.saturating_add(read_field!(p, loan));
    if read_field!(p, money) < 0 {
        amount = amount.saturating_add(neg(read_field!(p, money)));
    }
    for quarter in 0..4 {
        amount = amount.saturating_add(
            read_field!(p, old[quarter].income)
                .saturating_add(read_field!(p, old[quarter].expenses))
                .max(0)
                .saturating_mul(2),
        );
    }
    amount.max(1)
}
fn rating(s: &Services, p: *mut Finances, id: u8, update: bool) -> i32 {
    let mut part = [0_i64; 10];
    let mut min_profit = 0;
    let mut first = true;
    let mut count = 0_u32;
    let mut kind = 0_u8;
    let mut from = 0_u32;
    loop {
        let v = (s.vehicle_next_owned)(id, from, &raw mut kind);
        if v == INVALID {
            break;
        }
        from = v.wrapping_add(1);
        if kind < 4 && (s.vehicle_is_primary)(v) {
            let x = (s.vehicle_profit)(v);
            if x.profit_last_year > 0 {
                count = count.wrapping_add(1);
            }
            if x.economy_age > PROFIT_MIN_AGE && (first || min_profit > x.profit_last_year) {
                min_profit = x.profit_last_year;
                first = false;
            }
        }
    }
    part[0] = i64::from(count);
    part[2] = (min_profit >> 8).max(0);
    let mut count = 0_u32;
    each(
        |from| (s.station_next_owned)(id, from),
        |st| {
            let t = (s.station_times)(st);
            if t.since_load <= 20 || t.since_unload <= 20 {
                count = count.wrapping_add(station_facility_count(s, station_id(st)));
            }
        },
    );
    part[1] = i64::from(count);
    let entries = usize::from(read_field!(p, valid)).min(12);
    if entries != 0 {
        let mut min = i64::MAX;
        let mut max = i64::MIN;
        for i in 0..entries {
            let income =
                read_field!(p, old[i].income).saturating_add(read_field!(p, old[i].expenses));
            min = min.min(income);
            max = max.max(income);
        }
        part[3] = min.max(0);
        part[4] = max;
    }
    for i in 0..usize::from(read_field!(p, valid)).min(4) {
        for c in 0..64 {
            part[5] = part[5].saturating_add(i64::from(read_field!(p, old[i].cargo[c])));
        }
    }
    for c in 0..64 {
        part[6] += i64::from(read_field!(p, old[0].cargo[c]) != 0);
    }
    part[7] = read_field!(p, money).max(0);
    part[8] = 250_000_i64.saturating_sub(read_field!(p, loan));
    let mut score = 0;
    let mut total = 0;
    for i in 0..9 {
        score += part[i].clamp(0, SCORE_NEEDED[i]) * SCORE_WEIGHT[i] / SCORE_NEEDED[i];
        total += SCORE_WEIGHT[i];
    }
    part[9] = score;
    unsafe {
        addr_of_mut!(SCORES)
            .cast::<[i64; 10]>()
            .add(usize::from(id))
            .write(part);
    }
    if total != 1000 {
        score = score * 1000 / total;
    }
    if update {
        write_field!(p, old[0].performance, score as i32);
        // SAFETY: immutable object table; no finance borrow spans HQ mutation.
        unsafe {
            crate::objects::openttd_rust_object_update_hq(
                (s.object_services)(),
                (s.hq_location)(id),
                score as u32,
            );
        }
        let v = value(s, p, id, true);
        write_field!(p, old[0].value, v);
    }
    (s.performance_detail_dirty)();
    score as i32
}
fn takeover_limit(s: &Services, big: u8, small: u8) -> bool {
    let a = (s.company_vehicle_counts)(big);
    let b = (s.company_vehicle_counts)(small);
    let l = (s.vehicle_limits)();
    let within = |x: u16, y: u16, max: u16| u32::from(x) + u32::from(y) <= u32::from(max);
    within(a.trains, b.trains, l.trains)
        && within(a.road, b.road, l.road)
        && within(a.ships, b.ships, l.ships)
        && within(a.aircraft, b.aircraft, l.aircraft)
}
fn count_ais(s: &Services) -> u8 {
    let mut n = 0_u8;
    companies(s, |_, id| {
        if (s.identity.is_ai)((s.identity.company)(crate::company_type::CompanyID(id))) {
            n = n.wrapping_add(1);
        }
    });
    n
}
fn bankruptcy(s: &Services, m: &Month, p: *mut Finances, id: u8) {
    if m.infinite_money {
        return;
    }
    if read_field!(p, money).saturating_sub(read_field!(p, loan)) >= neg(maxloan(p)) {
        let previous = u32::from(read_field!(p, bankruptcy)).div_ceil(3);
        write_field!(p, bankruptcy, 0);
        write_field!(p, asked, 0);
        if previous != 0 {
            (s.identity.admin_update)(crate::company_type::CompanyID(id));
        }
        return;
    }
    let month = read_field!(p, bankruptcy).wrapping_add(1);
    write_field!(p, bankruptcy, month);
    match month {
        0..=3 | 5 | 6 | 8 | 9 => {}
        4 => (s.company_in_trouble)(id),
        7 => {
            let v = value(s, p, id, false);
            write_field!(p, bankrupt_value, v);
            write_field!(p, asked, 1_u16 << id);
            write_field!(p, timeout, 0);
        }
        _ => {
            let networking = (s.finance.networking)();
            if !networking && (s.local_company)() == id {
                write_field!(p, asked, u16::MAX);
            } else if !networking || (s.network_server)() {
                // Singleplayer executes the deletion synchronously; `p` is dead after it.
                crate::command_post::delete_company(s.shared, id);
                return;
            }
        }
    }
    // Original subtraction promotes uint8_t to int before ceil division.
    if (i32::from(month) + 2) / 3 != (i32::from(month) - 1 + 2) / 3 {
        (s.identity.admin_update)(crate::company_type::CompanyID(id));
    }
}
fn maintenance(s: &Services, p: *mut Finances, id: u8) {
    let mut infra: Infrastructure = unsafe { std::mem::zeroed() };
    (s.company_infrastructure)(id, &raw mut infra);
    let mut cost = 0_i64;
    for rt in 0..64 {
        if infra.rail[rt] != 0 {
            cost = cost.saturating_add((s.rail_maintenance_cost)(
                rt as u8,
                infra.rail[rt],
                infra.rail_total,
            ));
        }
    }
    cost = cost.saturating_add((s.signal_maintenance_cost)(infra.signal));
    for rt in 0..63 {
        if infra.road[rt] != 0 {
            let total = if infra.road_is_road & (1 << rt) != 0 {
                infra.road_total
            } else {
                infra.tram_total
            };
            cost = cost.saturating_add((s.road_maintenance_cost)(rt as u8, infra.road[rt], total));
        }
    }
    cost = cost.saturating_add((s.canal_maintenance_cost)(infra.water));
    cost = cost.saturating_add((s.station_maintenance_cost)(infra.station));
    cost = cost.saturating_add(crate::station_lifecycle::airport_maintenance(
        unsafe { &*(s.station_lifecycle)() },
        id,
    ));
    subtract(s.finance, p, id, cost, EXPENSES_PROPERTY);
}
fn statistics(s: &Services, m: &Month) {
    companies(s, |p, id| bankruptcy(s, m, p, id));
    let current = (s.finance.current_company)();
    if m.maintenance {
        companies(s, |p, id| {
            (s.finance.set_current_company)(id);
            maintenance(s, p, id);
        });
    }
    (s.finance.set_current_company)(current);
    if ![0, 3, 6, 9].contains(&m.month) {
        return;
    }
    companies(s, |p, id| {
        for i in (1..24).rev() {
            write_field!(p, old[i], read_field!(p, old[i - 1]));
        }
        write_field!(p, old[0], read_field!(p, current));
        write_field!(p, current, unsafe { std::mem::zeroed() });
        if read_field!(p, valid) != 24 {
            write_field!(p, valid, read_field!(p, valid).wrapping_add(1));
        }
        rating(s, p, id, true);
        if read_field!(p, preview) != 0 {
            write_field!(p, preview, read_field!(p, preview).wrapping_sub(1));
        }
    });
    (s.company_graphs_dirty)();
}
fn inflation(check: bool, year: i32) -> bool {
    if check && !(ORIGINAL_BASE_YEAR..ORIGINAL_MAX_YEAR).contains(&year) {
        return true;
    }
    let prices = read_field!(e(), prices);
    let payment = read_field!(e(), payment);
    if prices == MAX_INFLATION || payment == MAX_INFLATION {
        return true;
    }
    write_field!(
        e(),
        prices,
        prices
            .wrapping_add(
                prices
                    .wrapping_mul(u64::from(read_field!(e(), infl_amount)))
                    .wrapping_mul(54)
                    >> 16
            )
            .min(MAX_INFLATION)
    );
    write_field!(
        e(),
        payment,
        payment
            .wrapping_add(
                payment
                    .wrapping_mul(u64::from(read_field!(e(), infl_payment)))
                    .wrapping_mul(54)
                    >> 16
            )
            .min(MAX_INFLATION)
    );
    false
}
fn recompute(s: &Services, es: &EconomySettings) {
    write_field!(
        e(),
        max_loan,
        ((u64::from(es.max_loan).wrapping_mul(read_field!(e(), prices)) >> 16) / 10_000 * 10_000)
            as i64
    );
    for i in 0..PRICE_COUNT {
        let spec = (s.price_base)(i as u32);
        let modif = match spec.category {
            1 => es.vehicle_costs,
            2 => es.construction_cost,
            _ => 1,
        };
        let mut price = spec.start_price.saturating_mul(match modif {
            0 => 6,
            1 => 8,
            2 => 9,
            _ => unreachable!(),
        });
        price = (price as u64).wrapping_mul(read_field!(e(), prices)) as i64;
        let shift = i32::from(unsafe { addr_of!(MULTIPLIERS).cast::<i8>().add(i).read() }) - 19;
        price = if shift >= 0 {
            price.wrapping_shl(shift as u32)
        } else {
            price >> (-shift)
        };
        if price == 0 {
            price = spec.start_price.clamp(-1, 1);
        }
        unsafe {
            addr_of_mut!(PRICES).cast::<i64>().add(i).write(price);
        }
    }
    let mut initial = 0_i64;
    let mut from = 0_u32;
    loop {
        let id = (s.cargo_next)(from, &raw mut initial);
        if id == INVALID {
            break;
        }
        from = id.wrapping_add(1);
        (s.set_cargo_payment)(
            id,
            initial.saturating_mul(read_field!(e(), payment) as i64) >> 16,
        );
    }
    (s.price_windows_dirty)();
}
fn interest(s: &Services, m: &Month) {
    let current = (s.finance.current_company)();
    companies(s, |p, id| {
        (s.finance.set_current_company)(id);
        let rate = i64::from(read_field!(e(), interest));
        let mut fee = read_field!(p, loan).saturating_mul(rate) / 100;
        let cash = available(p, m.infinite_money);
        if cash < 0 {
            fee = fee.saturating_add(neg(cash).saturating_mul(rate) / 100);
        }
        let month = i64::from(m.month);
        let previous = fee.saturating_mul(month) / 12;
        let now = fee.saturating_mul(month + 1) / 12;
        subtract(
            s.finance,
            p,
            id,
            now.saturating_sub(previous),
            EXPENSES_LOAN_INTEREST,
        );
        subtract(
            s.finance,
            p,
            id,
            unsafe { addr_of!(PRICES).cast::<i64>().read() } >> 2,
            usize::from(EXPENSES_OTHER),
        );
    });
    (s.finance.set_current_company)(current);
}
fn fluctuations(s: &Services, m: &Month) {
    if m.fluctuating {
        write_field!(e(), fluct, read_field!(e(), fluct).wrapping_sub(1));
    } else if read_field!(e(), fluct) <= 0 {
        write_field!(e(), fluct, -12);
    } else {
        return;
    }
    if read_field!(e(), fluct) == 0 {
        write_field!(e(), fluct, -((s.shared.random() & 3) as i16));
        (s.recession_news)(true);
    } else if read_field!(e(), fluct) == -12 {
        write_field!(e(), fluct, ((s.shared.random() & 255) + 312) as i16);
        (s.recession_news)(false);
    }
}
fn offer(s: &Services, p: *mut Finances, id: u8) {
    let timeout = read_field!(p, timeout);
    if timeout != 0 {
        let next = timeout.wrapping_sub(i16::from(MAX_COMPANIES));
        write_field!(p, timeout, next);
        if next <= 0 {
            write_field!(p, timeout, 0);
        }
        return;
    }
    if read_field!(p, asked) == u16::MAX {
        return;
    }
    let mut best = INVALID_OWNER;
    let mut performance = -1;
    companies(s, |q, other| {
        let score = read_field!(q, old[1].performance);
        if read_field!(q, asked) == 0
            && read_field!(p, asked) & (1 << other) == 0
            && performance < score
            && takeover_limit(s, other, id)
        {
            performance = score;
            best = other;
        }
    });
    if performance == -1 {
        write_field!(p, asked, u16::MAX);
        return;
    }
    write_field!(p, asked, read_field!(p, asked) | (1 << best));
    write_field!(
        p,
        timeout,
        (3 * 30 * DAY_TICKS / (i32::from(MAX_COMPANIES) - 1)) as i16
    );
    (s.ask_merger)(best, id, read_field!(p, bankrupt_value));
    if (s.is_interactive_company)(best) {
        (s.show_buy_company)(id);
    }
}
fn startup(s: &Services, f: &mut Startup) -> bool {
    if f.stage == 1 {
        f.stage = 2;
        (s.new_company_events)(f.company as u8);
        return false;
    }
    f.company = INVALID;
    if !(s.can_allocate_company)() {
        return false;
    }
    let colour = crate::company_identity::generate_colour(s.identity);
    if f.requested != INVALID_OWNER && !(s.finance.company_finances)(f.requested).is_null() {
        return false;
    }
    let c = (s.allocate_company)(f.requested, f.is_ai);
    f.company = c.id;
    let id = c.id as u8;
    let company = (s.identity.company)(crate::company_type::CompanyID(id));
    crate::company_identity::set_startup_colour(s.identity, company, colour);
    let loan = (((INITIAL_LOAN as u64).wrapping_mul(read_field!(e(), prices)) >> 16) / 10_000
        * 10_000) as i64;
    let loan = loan.min(read_field!(e(), max_loan));
    write_field!(c.finances, loan, loan);
    write_field!(c.finances, money, loan);
    if (s.prepare_new_company)(id, f.is_ai) {
        crate::company_manager_face::randomise_company_manager_face(
            s.identity.faces,
            (s.identity.face)(company),
            s.identity.randomizer,
        );
    }
    (s.company_defaults)(id);
    crate::company_identity::generate_president(s.identity, company);
    (s.startup_windows)();
    if f.is_ai && (!(s.finance.networking)() || (s.network_server)()) {
        f.stage = 1;
        return true;
    }
    (s.new_company_events)(id);
    false
}
fn transfer(s: &Services, ownership: &crate::tile_owner::Context, old: u8, new: u8) {
    let f = s.finance;
    let current = (f.current_company)();
    (f.set_current_company)(old);
    if (f.networking)() {
        (s.clients_to_spectators)(old);
    }
    if (s.local_company)() == old {
        let saved = (f.current_company)();
        let mut from = 0_u32;
        loop {
            let c = (s.company_next)(from);
            if c.id == INVALID {
                break;
            }
            from = c.id.wrapping_add(1);
            if c.id as u8 != old {
                set_local_company(s, crate::company_type::CompanyID(c.id as u8));
                break;
            }
        }
        (f.set_current_company)(saved);
    }
    if new == INVALID_OWNER {
        write_field!((f.company_finances)(old), money, (u64::MAX >> 2) as i64);
    }
    unsafe { &*(s.subsidies)() }.transfer(CompanyID(old), CompanyID(new));
    let mut t = TownRights {
        rating: 0,
        have_ratings: 0,
        exclusive_counter: 0,
        exclusivity: 0,
    };
    let mut from = 0_u32;
    loop {
        let id = (s.town_next)(from, old, &raw mut t);
        if id == INVALID {
            break;
        }
        from = id.wrapping_add(1);
        if new != INVALID_OWNER && t.have_ratings & (1 << old) != 0 {
            let rating = if t.have_ratings & (1 << new) != 0 {
                (s.town_rating)(id, new).max(t.rating)
            } else {
                t.rating
            };
            (s.set_town_rating)(id, new, rating, true);
        }
        (s.set_town_rating)(id, old, 500, false);
        if t.exclusive_counter > 0 && t.exclusivity == old {
            if new == INVALID_OWNER {
                (s.set_town_exclusivity)(id, INVALID_OWNER, 0);
            } else {
                (s.set_town_exclusivity)(id, new, t.exclusive_counter);
            }
        }
    }
    let mut kind = 0_u8;
    let mut from = 0_u32;
    loop {
        let v = (s.vehicle_next_owned)(old, from, &raw mut kind);
        if v == INVALID {
            break;
        }
        from = v.wrapping_add(1);
        if kind >= 4 {
            continue;
        }
        if new == INVALID_OWNER {
            if (s.vehicle_previous)(v) == INVALID {
                (s.delete_vehicle)(v);
            }
        } else {
            let g = (s.vehicle_group_flags)(v);
            if g.engine_countable {
                (s.count_group_engine)(v, -1);
            }
            if g.primary {
                (s.count_group_vehicle)(v, -1);
            }
        }
    }
    (s.remove_engine_replacements)(old);
    each(
        |from| (s.group_next_owned)(old, from),
        |id| {
            if new == INVALID_OWNER {
                (s.delete_group)(id);
            } else {
                (s.transfer_group)(id, new);
            }
        },
    );
    if new != INVALID_OWNER {
        (s.copy_service_interval_defaults)(old, new);
    }
    let mut from = 0_u32;
    loop {
        let v = (s.vehicle_next_owned)(old, from, &raw mut kind);
        if v == INVALID {
            break;
        }
        from = v.wrapping_add(1);
        if kind >= 4 {
            continue;
        }
        if !(s.vehicle_service_interval_is_custom)(v) {
            let settings = (s.settings)();
            let vehicle = unsafe { (settings.transactions.groups.vehicle)(v) };
            let defaults = (settings.company_vehicle_defaults)(crate::company_type::CompanyID(new));
            let kind = crate::fleet::record(settings.transactions.groups, vehicle).vehicle_type();
            let interval = service_interval_from(
                settings.shared,
                defaults,
                crate::vehicle_type::VehicleType(kind),
            );
            let percent = read_field!(defaults, servint_ispercent);
            (s.reset_service_interval)(v, interval, percent);
        }
        (s.set_vehicle_owner)(v, new);
        let g = (s.vehicle_group_flags)(v);
        if g.engine_countable {
            (s.count_group_engine)(v, 1);
        }
        if g.primary {
            (s.count_group_vehicle)(v, 1);
            (s.assign_unit_number)(v, new);
        }
    }
    if new != INVALID_OWNER {
        (s.update_autoreplace)(new);
    }
    let map_size = s.shared.map_size();
    for tile in 0..map_size {
        // SAFETY: The outer C++ frame owns live scratch across this traversal.
        unsafe { change_tile_owner(ownership, TileIndex(tile), CompanyID(old), CompanyID(new)) };
    }
    if new != INVALID_OWNER {
        for tile in 0..map_size {
            // `IsTileType(MP_RAILWAY)`, then `IsLevelCrossingTile` off rail; only those two
            // kinds read `GetTileOwner`. `HasSignals` follows the owner test.
            let world = crate::map_func::World::new(s.shared);
            let railway = world.kind(tile) == crate::tile_type::MP_RAILWAY;
            let crossing = !railway && world.level_crossing_tile(tile);
            let owner = if railway || crossing {
                world.owner(tile)
            } else {
                0
            };
            if railway && owner == new && world.has_signals(tile) {
                let mut tracks = world.tracks(tile);
                loop {
                    // RemoveFirstTrack: INVALID_TRACK leaves NONE/INVALID bits unchanged.
                    let track = if tracks != 0 && tracks != 0xFF {
                        let first = tracks.trailing_zeros() as u8;
                        tracks &= tracks - 1;
                        first
                    } else {
                        0xFF
                    };
                    if crate::map_func::World::new(s.shared).has_signal_on_track(tile, track) {
                        (s.add_track_to_signal_buffer)(tile, track, new);
                    }
                    if tracks == 0 {
                        break;
                    }
                }
            } else if crossing && owner == new {
                (s.shared.update_level_crossing)(tile, true);
            }
        }
    }
    (s.update_signals_in_buffer)();
    if new != INVALID_OWNER {
        (s.add_airport_infrastructure)(new, old);
    }
    let owner = if new == INVALID_OWNER {
        OWNER_NONE
    } else {
        new
    };
    each(
        |from| (s.station_next_owned)(old, from),
        |id| (s.set_station_owner)(id, owner),
    );
    each(
        |from| (s.waypoint_next_owned)(old, from),
        |id| (s.set_waypoint_owner)(id, owner),
    );
    each(
        |from| (s.sign_next_owned)(old, from),
        |id| (s.set_sign_owner)(id, owner),
    );
    each(
        |from| (s.goal_next_owned)(old, from),
        |id| (s.delete_goal)(id),
    );
    crate::cargomonitor::openttd_rust_monitor_clear_pickups(old);
    crate::cargomonitor::openttd_rust_monitor_clear_deliveries(old);
    each(
        |from| (s.story_page_next_owned)(old, from),
        |id| {
            unsafe {
                crate::objectives::delete_elements(
                    (s.objectives)(),
                    crate::story_type::StoryPageID(id as u16),
                );
            }
            (s.delete_story_page)(id);
        },
    );
    if new != INVALID_OWNER {
        (s.change_window_owner)(old, new);
    }
    (f.set_current_company)(current);
    (s.shared.screen_dirty)();
}
pub(crate) fn control(
    s: &Services,
    ownership: &crate::tile_owner::Context,
    c: &mut Control,
) -> bool {
    if c.stage == 0 {
        (s.company_league_dirty)();
        match c.action {
            crate::company_type::CCA_NEW => {
                if !(s.finance.networking)() {
                    c.result = error();
                    return false;
                }
                if !c.execute {
                    c.result = done(0, NO_EXPENSE);
                    return false;
                }
                // The client is looked up again by ID; no client leaves meanwhile.
                (s.close_network_status)();
                let mut startup_frame = Startup {
                    company: INVALID,
                    requested: INVALID_OWNER,
                    is_ai: false,
                    stage: 0,
                };
                // A human company never requests AI::StartNew.
                startup(s, &mut startup_frame);
                if startup_frame.company == INVALID {
                    (s.network_spectate)(c.client);
                } else {
                    let id = startup_frame.company as u8;
                    network_new(s, id, Some(c.client));
                    let company = (s.network_own_company)(id, c.client);
                    if !company.is_null() {
                        set_local_company(s, (s.identity.company_id)(company));
                        (s.network_own_company_settings)(company);
                    }
                }
            }
            crate::company_type::CCA_NEW_AI => {
                let target = c.target;
                if target != INVALID_OWNER && u32::from(target) >= u32::from(MAX_COMPANIES) {
                    c.result = error();
                    return false;
                }
                if !(s.finance.networking)()
                    && target != INVALID_OWNER
                    && !(s.finance.company_finances)(target).is_null()
                {
                    c.result = error();
                    return false;
                }
                if !c.execute {
                    c.result = done(0, NO_EXPENSE);
                    return false;
                }
                (s.assert_new_ai_slot)(target);
                c.startup = Startup {
                    company: INVALID,
                    requested: target,
                    is_ai: true,
                    stage: 0,
                };
                c.stage = 1;
            }
            crate::company_type::CCA_DELETE => {
                let target = c.target;
                if c.reason >= 3 {
                    c.result = error();
                    return false;
                }
                if !(s.finance.networking)() && (s.company_count)() == 1 {
                    c.result = error();
                    return false;
                }
                if (s.finance.company_finances)(target).is_null() {
                    c.result = error();
                    return false;
                }
                if !c.execute {
                    c.result = done(0, NO_EXPENSE);
                    return false;
                }
                (s.company_bankrupt_news)(target);
                transfer(s, ownership, target, INVALID_OWNER);
                if (s.identity.is_ai)((s.identity.company)(crate::company_type::CompanyID(target)))
                {
                    (s.stop_ai)(target);
                }
                (s.delete_company)(target);
                (s.company_removed)(target, c.reason);
            }
            _ => {
                c.result = error();
                return false;
            }
        }
    }
    if c.stage == 1 {
        if startup(s, &mut c.startup) {
            return true;
        }
        c.stage = 2;
        if c.startup.company != INVALID {
            network_new(s, c.startup.company as u8, None);
        }
    }
    (s.company_ctrl_windows)();
    c.result = done(0, NO_EXPENSE);
    false
}
/// One iteration of the original `for (i = 0; i < max; i++)` posting loop; the
/// increment precedes the check of the resumed iteration.
fn post_competitor(s: &Services, t: &mut Tick) -> bool {
    let settings = s.shared.settings();
    if t.index >= i32::from(unsafe { settings.max_competitors.read() }) {
        return false;
    }
    t.index += 1;
    if (s.finance.networking)() {
        let old = t.num_companies;
        t.num_companies = old.wrapping_add(1);
        if old >= usize::from(unsafe { settings.max_companies.read() }) {
            return false;
        }
    }
    let old = t.num_ais;
    t.num_ais = old.wrapping_add(1);
    old < unsafe { settings.max_competitors.read() }
}
fn tick(s: &Services, t: &mut Tick) -> bool {
    if t.stage == 0 {
        if crate::session::mode() == crate::openttd::GameMode::EDITOR {
            return false;
        }
        let id = unsafe { TICK } as u8;
        let finances = (s.finance.company_finances)(id);
        if !finances.is_null() {
            let company = (s.identity.company)(crate::company_type::CompanyID(id));
            if (s.identity.name_id)(company) != 0 {
                crate::company_identity::generate_name(s.identity, company);
            }
            if read_field!(finances, asked) != 0 {
                offer(s, finances, id);
            }
        }
        if (s.competitor_fired)()
            && crate::session::mode() != crate::openttd::GameMode::MENU
            && (s.ai_can_start)()
        {
            t.timeout = i32::from(unsafe { s.shared.settings().competitor_interval.read() })
                .wrapping_mul(60)
                .wrapping_mul(TICKS_PER_SECOND);
            if t.timeout == 0 {
                t.num_ais = count_ais(s);
                t.num_companies = (s.company_count)();
                t.index = 0;
                t.stage = 1;
            } else {
                t.stage = 2;
            }
        } else {
            t.stage = 3;
        }
    }
    if t.stage == 1 {
        if post_competitor(s, t) {
            return true;
        }
        t.timeout = 10 * 60 * TICKS_PER_SECOND;
        t.stage = 2;
    }
    if t.stage == 2 {
        let timeout = t.timeout;
        let timeout = timeout
            .wrapping_add((s.script_random_next)((timeout / 4) as u32) as i32)
            .wrapping_sub(timeout / 8);
        (s.reset_competitor_timeout)(timeout.max(1) as u32);
    }
    t.stage = 4;
    unsafe {
        TICK = TICK.wrapping_add(1) % u32::from(MAX_COMPANIES);
    }
    false
}
fn buy(
    s: &Services,
    ownership: &crate::tile_owner::Context,
    id: u8,
    mut hostile: bool,
    execute: bool,
) -> Cost {
    let p = (s.finance.company_finances)(id);
    if p.is_null() {
        return error();
    }
    let current = (s.finance.current_company)();
    // Special owners lie outside the 16-bit CompanyMask and have no offer.
    let asked = read_field!(p, asked) & 1_u16.checked_shl(u32::from(current)).unwrap_or(0) != 0;
    if hostile && asked {
        hostile = false;
    }
    if !hostile && !asked {
        return error();
    }
    if hostile && !(s.identity.is_ai)((s.identity.company)(crate::company_type::CompanyID(id))) {
        return error();
    }
    let networking = (s.finance.networking)();
    if hostile && networking {
        return error();
    }
    if !networking && (s.local_company)() == id {
        return error();
    }
    if id == current {
        return error();
    }
    if !takeover_limit(s, current, id) {
        return failed(6, 0);
    }
    let cost = if hostile {
        hostile_value(s, p, id)
    } else {
        read_field!(p, bankrupt_value)
    };
    if execute {
        (s.merger_news)(id, hostile);
        transfer(s, ownership, id, current);
        if (s.identity.is_ai)((s.identity.company)(crate::company_type::CompanyID(id))) {
            (s.stop_ai)(id);
        }
        (s.acquisition_windows)(id);
        (s.delete_company)(id);
    }
    done(cost, EXPENSES_OTHER)
}
fn loan(
    f: &FinanceServices,
    increase: bool,
    cmd: u8,
    amount: i64,
    infinite: bool,
    execute: bool,
) -> Cost {
    let id = (f.current_company)();
    let p = (f.company_finances)(id);
    let current = read_field!(p, loan);
    let max = maxloan(p);
    if increase && current >= max {
        return failed(2, max);
    }
    if !increase && current == 0 {
        return failed(3, 0);
    }
    let mut loan = match cmd {
        0 => {
            if increase {
                LOAN_INTERVAL
            } else {
                current.min(LOAN_INTERVAL)
            }
        }
        1 => {
            if increase {
                max.saturating_sub(current)
            } else {
                current.min(available(p, infinite)).max(LOAN_INTERVAL)
            }
        }
        2 => amount,
        _ => return error(),
    };
    if cmd == 2 {
        if increase {
            if loan < LOAN_INTERVAL
                || current.saturating_add(loan) > max
                || loan % LOAN_INTERVAL != 0
            {
                return error();
            }
        } else if loan % LOAN_INTERVAL != 0 || loan < LOAN_INTERVAL || loan > current {
            return error();
        }
    }
    if !increase && cmd == 1 {
        loan = loan.saturating_sub(loan % LOAN_INTERVAL);
    }
    if increase && read_field!(p, money) > i64::MAX.saturating_sub(loan) {
        return error();
    }
    if !increase && available(p, infinite) < loan {
        return failed(4, loan);
    }
    if execute {
        if increase {
            write_field!(p, money, read_field!(p, money).saturating_add(loan));
            write_field!(p, loan, current.saturating_add(loan));
        } else {
            write_field!(p, money, read_field!(p, money).saturating_sub(loan));
            write_field!(p, loan, current.saturating_sub(loan));
        }
        (f.invalidate_company_windows)(id);
    }
    done(0, if increase { EXPENSES_OTHER } else { NO_EXPENSE })
}

// Entry points. All run serially on the game thread. Table and frame pointers are
// live for the call; finance pointers are live canonical owners resolved once by
// the facade or through `company_finances`. Panics abort (`panic = "abort"`).

/// `SubtractMoneyFromAnyCompany`.
/// # Safety
/// `p` is the live finance owner of company `id`.
pub(crate) unsafe fn subtract_money(
    f: &FinanceServices,
    p: *mut Finances,
    id: u8,
    cost: i64,
    expense: u8,
) {
    subtract(f, p, id, cost, usize::from(expense));
}
/// `SubtractMoneyFromCompanyFract`.
/// # Safety
/// `p` is the live finance owner of company `id`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_company_subtract_fraction(
    f: &FinanceServices,
    p: *mut Finances,
    id: u8,
    cost: i64,
    expense: u8,
) {
    subtract_fraction(f, p, id, cost, usize::from(expense));
}
/// `UpdateLandscapingLimits`.
pub(crate) fn update_landscaping_limits(
    s: &Services,
    r: *const crate::settings_type::ConstructionLimits,
) {
    let next = |value: u32, per: u32, burst: u16| -> u32 {
        (u64::from(value) + u64::from(per)).min(u64::from(burst) << 16) as u32
    };
    companies(s, |p, _| {
        write_field!(
            p,
            terraform,
            next(
                read_field!(p, terraform),
                read_field!(r, terraform_per_64k_frames),
                read_field!(r, terraform_frame_burst)
            )
        );
        write_field!(
            p,
            clear,
            next(
                read_field!(p, clear),
                read_field!(r, clear_per_64k_frames),
                read_field!(r, clear_frame_burst)
            )
        );
        write_field!(
            p,
            tree,
            next(
                read_field!(p, tree),
                read_field!(r, tree_per_64k_frames),
                read_field!(r, tree_frame_burst)
            )
        );
        write_field!(
            p,
            object,
            next(
                read_field!(p, object),
                read_field!(r, build_object_per_64k_frames),
                read_field!(r, build_object_frame_burst)
            )
        );
    });
}
/// `CalculateCompanyValue`.
/// # Safety
/// `p` is the live finance owner of company `id`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_company_value(
    s: &Services,
    p: *const Finances,
    id: u8,
    including_loan: bool,
) -> i64 {
    value(s, p.cast_mut(), id, including_loan)
}
/// `CalculateHostileTakeoverValue`.
/// # Safety
/// `p` is the live finance owner of company `id`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_company_hostile_takeover_value(
    s: &Services,
    p: *const Finances,
    id: u8,
) -> i64 {
    hostile_value(s, p.cast_mut(), id)
}
/// `UpdateCompanyRatingAndValue`.
/// # Safety
/// `p` is the live finance owner of company `id`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_company_update_rating(
    s: &Services,
    p: *mut Finances,
    id: u8,
    update: bool,
) -> i32 {
    rating(s, p, id, update)
}
/// Economy month: `CompaniesGenStatistics`, `CompaniesPayInterest`,
/// `HandleEconomyFluctuations`.
pub(crate) fn openttd_rust_economy_month(s: &Services, m: &Month) {
    statistics(s, m);
    interest(s, m);
    fluctuations(s, m);
}
/// `AddInflation`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_economy_add_inflation_abi(check_year: bool, year: i32) -> u8 {
    u8::from(inflation(check_year, year))
}
/// `RecomputePrices`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_economy_recompute_prices(s: &Services, es: &EconomySettings) {
    recompute(s, es);
}
/// Calendar-month inflation timer body.
pub(crate) fn openttd_rust_economy_calendar_month(s: &Services, es: &EconomySettings) {
    if es.inflation {
        inflation(true, es.year);
        recompute(s, es);
    }
}
/// `ResetPriceBaseMultipliers`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_economy_reset_price_multipliers() {
    unsafe {
        MULTIPLIERS = [0; PRICE_COUNT];
    }
}
/// `SetPriceBaseMultiplier`; the facade keeps the original `price < PR_END` assert.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_economy_set_price_multiplier(price: u32, factor: i32) {
    unsafe {
        (*addr_of_mut!(MULTIPLIERS))[price as usize] = factor.clamp(-8, 16) as i8;
    }
}
/// `StartupEconomy`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_economy_startup(s: &Services, es: &EconomySettings) {
    write_field!(e(), interest, es.initial_interest);
    write_field!(e(), infl_amount, es.initial_interest);
    write_field!(e(), infl_payment, es.initial_interest.saturating_sub(1));
    write_field!(e(), fluct, ((s.shared.random() & 255) + 168) as i16);
    if es.inflation {
        let months = (es.year.min(ORIGINAL_MAX_YEAR) - ORIGINAL_BASE_YEAR) * 12;
        for _ in 0..months {
            inflation(false, es.year);
        }
    }
    recompute(s, es);
    (s.industry_daily_changes)(true);
}
/// `InitializeEconomy`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_economy_initialize() {
    write_field!(e(), prices, 1 << 16);
    write_field!(e(), payment, 1 << 16);
    crate::cargomonitor::openttd_rust_monitor_clear_pickups(crate::company_type::INVALID_OWNER);
    crate::cargomonitor::openttd_rust_monitor_clear_deliveries(crate::company_type::INVALID_OWNER);
}
/// `GetPrice` after the facade applies the `PR_END` check and the `NewGRF` shift.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_economy_price(index: u32, cost_factor: u32, shift: i32) -> i64 {
    let Some(&base) = (unsafe { &*addr_of!(PRICES) }).get(index as usize) else {
        return 0;
    };
    let price = base.saturating_mul(i64::from(cost_factor));
    if shift >= 0 {
        price.wrapping_shl(shift as u32)
    } else {
        price >> (-shift)
    }
}
/// `GetAvailableMoney`; `p` is null for an invalid company.
/// # Safety
/// `p` is null or a live finance owner.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_company_available_money(
    p: *const Finances,
    infinite: bool,
) -> i64 {
    available(p, infinite)
}
/// Original positive-cost finance predicate; null denotes an invalid company.
/// # Safety
/// `p` is null or a live finance owner.
pub(crate) unsafe fn has_money(p: *const Finances, infinite: bool, cost: i64) -> bool {
    cost <= 0 || infinite || p.is_null() || cost <= read_field!(p, money)
}
/// `Company::GetMaxLoan`.
/// # Safety
/// `p` is a live finance owner.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_company_max_loan(p: *const Finances) -> i64 {
    maxloan(p)
}
/// `DoStartupNewCompany`; `true` asks the caller to run `AI::StartNew(company)`
/// and call again. If that throws, the frame is simply abandoned.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_startup_abi(s: &Services, f: &mut Startup) -> u8 {
    u8::from(startup(s, f))
}
/// Competitor timeout callback; `true` asks the caller to Post `CCA_NEW_AI`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_competitor_timeout_abi(s: &Services) -> u8 {
    let settings = s.shared.settings();
    if crate::session::mode() == crate::openttd::GameMode::MENU || !(s.ai_can_start)() {
        return u8::from(false);
    }
    if (s.finance.networking)()
        && (s.company_count)() >= usize::from(unsafe { settings.max_companies.read() })
    {
        return u8::from(false);
    }
    if unsafe { settings.competitor_interval.read() } == 0 {
        return u8::from(false);
    }
    u8::from(count_ais(s) < unsafe { settings.max_competitors.read() })
}
/// `OnTick_Companies`; `true` asks the caller to Post `CCA_NEW_AI` and call again.
/// # Safety
/// Services address the live serialized game and native Post resumes only after Rust returns.
pub(crate) unsafe fn on_tick(s: &Services, t: &mut Tick) -> bool {
    tick(s, t)
}
/// Yearly company statistics timer body.
/// # Safety
/// `y.local_finances` is the live owner of `y.local` when that is a company.
pub(crate) unsafe fn openttd_rust_company_yearly(s: &Services, y: &Year) {
    companies(s, |p, id| {
        write_field!(p, expenses[2], read_field!(p, expenses[1]));
        write_field!(p, expenses[1], read_field!(p, expenses[0]));
        write_field!(p, expenses[0], [0; 13]);
        (s.finances_dirty)(id);
    });
    if y.show_finances && y.local != INVALID_OWNER {
        (s.show_company_finances)(y.local);
        let p = y.local_finances;
        let bad = read_field!(p, valid) > 5
            && read_field!(p, old[0].performance) < read_field!(p, old[4].performance);
        if y.new_year_sound {
            (s.new_year_sound)(bad);
        }
    }
}
/// `CheckTakeoverVehicleLimit`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_takeover_allowed_abi(
    s: &Services,
    big: u8,
    small: u8,
) -> u8 {
    u8::from(takeover_limit(s, big, small))
}
/// `CmdCompanyCtrl`; `true` asks the caller to run `AI::StartNew(startup.company)`
/// and call again. The result is in `result` once it returns `false`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_control_abi(
    s: &Services,
    ownership: &crate::tile_owner::Context,
    c: &mut Control,
) -> u8 {
    u8::from(control(s, ownership, c))
}
/// `ChangeOwnershipOfCompanyItems`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_change_ownership(
    s: &Services,
    ownership: &crate::tile_owner::Context,
    old: u8,
    new: u8,
) {
    transfer(s, ownership, old, new);
}
/// `CmdBuyCompany`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_buy(
    s: &Services,
    ownership: &crate::tile_owner::Context,
    id: u8,
    hostile: bool,
    execute: bool,
) -> Cost {
    buy(s, ownership, id, hostile, execute)
}
/// `CmdGiveMoney`; `give_money` is the economy setting.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_give_money(
    f: &FinanceServices,
    give_money: bool,
    money: i64,
    dest: u8,
    execute: bool,
) -> Cost {
    if !give_money {
        return error();
    }
    let current = (f.current_company)();
    let p = (f.company_finances)(current);
    let amount = money.min(20_000_000);
    if read_field!(p, money).saturating_sub(read_field!(p, loan)) < amount || amount < 0 {
        return failed(5, 0);
    }
    let q = (f.company_finances)(dest);
    if q.is_null() {
        return error();
    }
    if execute {
        (f.set_current_company)(dest);
        subtract(f, q, dest, neg(amount), usize::from(EXPENSES_OTHER));
        (f.set_current_company)(current);
        if (f.networking)() {
            (f.give_money_message)(dest, amount);
        }
    }
    done(amount, EXPENSES_OTHER)
}
/// `CmdIncreaseLoan`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_increase_loan(
    f: &FinanceServices,
    cmd: u8,
    amount: i64,
    infinite: bool,
    execute: bool,
) -> Cost {
    loan(f, true, cmd, amount, infinite, execute)
}
/// `CmdDecreaseLoan`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_decrease_loan(
    f: &FinanceServices,
    cmd: u8,
    amount: i64,
    infinite: bool,
    execute: bool,
) -> Cost {
    loan(f, false, cmd, amount, infinite, execute)
}
/// `CmdSetCompanyMaxLoan`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_set_max_loan(
    f: &FinanceServices,
    id: u8,
    amount: i64,
    execute: bool,
) -> Cost {
    if (f.current_company)() != OWNER_DEITY {
        return error();
    }
    if amount != i64::MIN && !(0..=MAX_LOAN_LIMIT).contains(&amount) {
        return error();
    }
    let p = (f.company_finances)(id);
    if p.is_null() {
        return error();
    }
    if execute {
        let amount = if amount == i64::MIN {
            amount
        } else {
            amount - amount % LOAN_INTERVAL
        };
        write_field!(p, max_loan, amount);
        (f.invalidate_company_windows)(id);
    }
    done(0, NO_EXPENSE)
}
/// `CmdChangeBankBalance`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_change_bank_balance(
    f: &FinanceServices,
    tile: u32,
    delta: i64,
    id: u8,
    expense: u8,
    execute: bool,
) -> Cost {
    let p = (f.company_finances)(id);
    if p.is_null() || expense >= EXPENSES_END {
        return error();
    }
    let current = (f.current_company)();
    if current != OWNER_DEITY {
        return error();
    }
    if execute {
        (f.set_current_company)(id);
        subtract(f, p, id, neg(delta), usize::from(expense));
        (f.set_current_company)(current);
        if tile != 0 {
            (f.money_animation)(tile, neg(delta));
        }
    }
    done(0, expense)
}

/// Initialize the four canonical construction budgets at Company construction.
/// # Safety
/// State is a live exclusive finance owner whose C++ view lifetime has started.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_company_initialize(
    p: *mut Finances,
    t: u32,
    c: u32,
    tree: u32,
    o: u32,
) {
    write_field!(p, terraform, t.wrapping_shl(16));
    write_field!(p, clear, c.wrapping_shl(16));
    write_field!(p, tree, tree.wrapping_shl(16));
    write_field!(p, object, o.wrapping_shl(16));
}

crate::abi::layouts! {
    fn abi_layout;
    "CompanyEconomyEntry" ["company_economy"] => Entry {
        income,
        expenses,
        cargo => delivered_cargo,
        performance => performance_history,
        value => company_value,
    };
    "CompanyFinances" ["company_economy"] => Finances {
        money,
        fraction => money_fraction,
        loan => current_loan,
        max_loan,
        preview => block_preview,
        empty => months_empty,
        bankruptcy => months_of_bankruptcy,
        asked => bankrupt_asked,
        timeout => bankrupt_timeout,
        bankrupt_value,
        terraform => terraform_limit,
        clear => clear_limit,
        tree => tree_limit,
        object => build_object_limit,
        expenses => yearly_expenses,
        current => cur_economy,
        old => old_economy,
        valid => num_valid_stat_ent,
    };
    "Economy" ["company_economy"] => Economy {
        max_loan,
        fluct,
        interest => interest_rate,
        infl_amount,
        infl_payment => infl_amount_pr,
        prices => inflation_prices,
        payment => inflation_payment,
        old_loan => old_max_loan_unround,
        old_fraction => old_max_loan_unround_fract,
    };
    "OpenTTDCompanyFinanceServices" ["company_economy"] => FinanceServices {
        current_company,
        set_current_company,
        networking,
        company_finances,
        invalidate_company_windows,
        give_money_message,
        money_animation,
    };
    "OpenTTDCompanyServices" ["company_economy"] => Services {
        identity,
        finance,
        shared,
        ai_can_start,
        allow_list_add,
        allow_list_remove,
        assign_company_context,
        close_construction_windows,
        cost,
        objectives,
        owned_by,
        settings,
        company_next,
        company_count,
        competitor_fired,
        can_allocate_company,
        local_company,
        network_server,
        company_vehicle_counts,
        vehicle_limits,
        company_in_trouble,
        hq_location,
        object_services,
        performance_detail_dirty,
        company_graphs_dirty,
        company_infrastructure,
        rail_maintenance_cost,
        signal_maintenance_cost,
        road_maintenance_cost,
        canal_maintenance_cost,
        station_maintenance_cost,
        station_lifecycle,
        recession_news,
        price_base,
        cargo_next,
        set_cargo_payment,
        price_windows_dirty,
        industry_daily_changes,
        ask_merger,
        is_interactive_company,
        show_buy_company,
        script_random_next,
        reset_competitor_timeout,
        finances_dirty,
        show_company_finances,
        new_year_sound,
        allocate_company,
        prepare_new_company,
        company_defaults,
        startup_windows,
        new_company_events,
        company_league_dirty,
        company_ctrl_windows,
        close_network_status,
        network_spectate,
        admin_company_new,
        network_own_company,
        network_own_company_settings,
        assert_new_ai_slot,
        company_bankrupt_news,
        stop_ai,
        delete_company,
        company_removed,
        merger_news,
        acquisition_windows,
        clients_to_spectators,
        subsidies,
        town_next,
        town_rating,
        set_town_rating,
        set_town_exclusivity,
        vehicle_next_owned,
        vehicle_value,
        vehicle_is_primary,
        vehicle_profit,
        vehicle_previous,
        vehicle_group_flags,
        vehicle_service_interval_is_custom,
        delete_vehicle,
        count_group_engine,
        count_group_vehicle,
        reset_service_interval,
        set_vehicle_owner,
        assign_unit_number,
        remove_engine_replacements,
        group_next_owned,
        delete_group,
        transfer_group,
        copy_service_interval_defaults,
        update_autoreplace,
        add_track_to_signal_buffer,
        update_signals_in_buffer,
        add_airport_infrastructure,
        station_next_owned,
        station_times,
        set_station_owner,
        waypoint_next_owned,
        set_waypoint_owner,
        sign_next_owned,
        set_sign_owner,
        goal_next_owned,
        delete_goal,
        story_page_next_owned,
        delete_story_page,
        change_window_owner,
    };
    "OpenTTDCompanyRef" ["company_economy"] => CompanyRef {
        finances,
        id,
    };
    "OpenTTDCompanyVehicleCounts" ["company_economy"] => VehicleCounts {
        trains,
        road,
        ships,
        aircraft,
    };
    "OpenTTDCompanyInfrastructure" ["company_economy"] => Infrastructure {
        rail,
        road,
        rail_total,
        road_total,
        tram_total,
        signal,
        water,
        station,
        road_is_road,
    };
    "OpenTTDCompanyVehicleGroupFlags" ["company_economy"] => VehicleGroupFlags {
        engine_countable,
        primary,
    };
    "OpenTTDCompanyVehicleProfit" ["company_economy"] => VehicleProfit {
        profit_last_year,
        economy_age,
    };
    "OpenTTDCompanyStationTimes" ["company_economy"] => StationTimes {
        since_load,
        since_unload,
    };
    "OpenTTDCompanyTownRights" ["company_economy"] => TownRights {
        rating,
        have_ratings,
        exclusive_counter,
        exclusivity,
    };
    "OpenTTDCompanyPriceBase" ["company_economy"] => PriceBase {
        start_price,
        category,
    };
    "OpenTTDCompanyCost" ["company_economy"] => Cost {
        cost,
        param,
        error,
        expense,
    };
    "OpenTTDCompanyStartup" ["company_economy"] => Startup {
        company,
        requested,
        is_ai,
        stage,
    };
    "OpenTTDCompanyControl" ["company_economy"] => Control {
        result,
        startup,
        client,
        action,
        target,
        reason,
        execute,
        stage,
    };
    "OpenTTDEconomyMonth" ["company_economy"] => Month {
        month,
        infinite_money,
        maintenance,
        fluctuating,
    };
    "OpenTTDCompanyYear" ["company_economy"] => Year {
        local_finances,
        local,
        show_finances,
        new_year_sound,
    };
    "OpenTTDEconomySettings" ["company_economy"] => EconomySettings {
        year,
        max_loan,
        initial_interest,
        vehicle_costs,
        construction_cost,
        inflation,
    };
}

fn network_new(s: &Services, id: u8, client: Option<u32>) {
    (s.admin_company_new)(id);
    let f = ((s.shared.command_post)().network)();
    let f = (f.network)();
    let company = (s.identity.company)(crate::company_type::CompanyID(id));
    let info = client.map_or(std::ptr::null_mut(), |id| {
        (s.shared.network_client_info)(id)
    });
    crate::network_lifecycle::new_company(f.lifecycle, f, company, info);
}

#[cfg(test)]
mod financial_command_tests {
    use super::*;
    use std::cell::RefCell;
    use std::sync::Mutex;

    const MONEY: [i64; 7] = [i64::MIN, -1, 0, 9_999, 10_000, 30_000_000, i64::MAX];
    const LOANS: [i64; 4] = [0, 100_000, 300_000, i64::MAX];
    const AMOUNTS: [i64; 9] = [
        i64::MIN,
        -1,
        0,
        9_999,
        10_000,
        10_001,
        20_000_001,
        2_000_000_001,
        i64::MAX,
    ];
    const VARIANTS: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 255];
    // Serializes Rust unit tests that access the process-wide economy state. A test
    // that writes it holds this lock for its entire run and restores the prior value.
    pub(super) static ECONOMY_LOCK: Mutex<()> = Mutex::new(());

    struct Fixture {
        owners: [*mut Finances; 3],
        current: u8,
        networking: bool,
        trace: Vec<[i64; 3]>,
    }
    thread_local! {
        static FIXTURE: RefCell<Fixture> = const { RefCell::new(Fixture {
            owners: [std::ptr::null_mut(); 3],
            current: 0,
            networking: false,
            trace: Vec::new(),
        }) };
    }

    extern "C" fn current_company() -> u8 {
        FIXTURE.with_borrow(|f| f.current)
    }
    extern "C" fn set_current_company(id: u8) {
        FIXTURE.with_borrow_mut(|f| f.current = id);
    }
    extern "C" fn networking() -> bool {
        FIXTURE.with_borrow(|f| f.networking)
    }
    extern "C" fn company_finances(id: u8) -> *mut Finances {
        FIXTURE.with_borrow(|f| {
            f.owners
                .get(usize::from(id))
                .copied()
                .unwrap_or(std::ptr::null_mut())
        })
    }
    extern "C" fn invalidate(id: u8) {
        FIXTURE.with_borrow_mut(|f| f.trace.push([3, i64::from(id), 0]));
    }
    extern "C" fn message(id: u8, amount: i64) {
        FIXTURE.with_borrow_mut(|f| f.trace.push([52, i64::from(id), amount]));
    }
    extern "C" fn animation(tile: u32, amount: i64) {
        FIXTURE.with_borrow_mut(|f| f.trace.push([53, i64::from(tile), amount]));
    }
    // Real typed production table; no shared defaults or layout offsets.
    const FINANCE: FinanceServices = FinanceServices {
        current_company,
        set_current_company,
        networking,
        company_finances,
        invalidate_company_windows: invalidate,
        give_money_message: message,
        money_animation: animation,
    };

    struct Run {
        old_max_loan: i64,
    }
    impl Run {
        fn start() -> Self {
            FIXTURE.with_borrow_mut(|f| {
                assert!(f.owners.iter().all(|p| p.is_null()));
                f.owners = std::array::from_fn(|_| openttd_rust_company_state_create());
            });
            // SAFETY: ECONOMY_LOCK serializes every test's economy accesses.
            // No finance callback accesses economy storage or retains a borrow.
            let old_max_loan = unsafe { (*openttd_rust_economy_state()).max_loan };
            unsafe { (*openttd_rust_economy_state()).max_loan = 300_000 };
            Self { old_max_loan }
        }
    }
    impl Drop for Run {
        fn drop(&mut self) {
            FIXTURE.with_borrow_mut(|f| {
                for owner in std::mem::replace(&mut f.owners, [std::ptr::null_mut(); 3]) {
                    // SAFETY: Each create result is owned exclusively by this test;
                    // command calls and all raw finance accesses have finished.
                    unsafe { openttd_rust_company_state_destroy(owner) };
                }
                f.trace.clear();
            });
            // SAFETY: The caller still holds ECONOMY_LOCK; no owner is borrowed.
            unsafe { (*openttd_rust_economy_state()).max_loan = self.old_max_loan };
        }
    }

    struct Case {
        op: u8,
        execute: bool,
        money: i64,
        loan: i64,
        amount: i64,
    }
    impl Case {
        /// Row `index` of the original's nested op, execute, money, loan and amount loops.
        fn from_index(index: usize) -> Self {
            Self {
                op: 21 + (index / 504) as u8,
                execute: !(index / 252).is_multiple_of(2),
                money: MONEY[(index / 36) % 7],
                loan: LOANS[(index / 9) % 4],
                amount: AMOUNTS[index % 9],
            }
        }

        fn reset(&self, variant: u8) {
            FIXTURE.with_borrow_mut(|f| {
                f.current = if self.op >= 24 && variant != 0 {
                    OWNER_DEITY
                } else {
                    0
                };
                f.networking = variant == 12;
                f.trace.clear();
                for &owner in &f.owners {
                    // SAFETY: Finances contains only zero-valid scalar/array
                    // fields, no function pointers. Owners are exclusive and
                    // no command or callback runs until all resets finish.
                    unsafe {
                        owner.write(std::mem::zeroed());
                        (*owner).money = self.money;
                        (*owner).loan = self.loan;
                        (*owner).max_loan = if variant == 2 {
                            MAX_LOAN_LIMIT
                        } else {
                            i64::MIN
                        };
                    }
                }
            });
        }

        fn invoke(&self, variant: u8) -> Cost {
            let id = if (self.op == 21 || self.op >= 24) && variant == 255 {
                255
            } else {
                1
            };
            match self.op {
                21 => openttd_rust_company_give_money(
                    &FINANCE,
                    variant != 13,
                    self.amount,
                    id,
                    self.execute,
                ),
                22 => openttd_rust_company_increase_loan(
                    &FINANCE,
                    variant,
                    self.amount,
                    variant == 3,
                    self.execute,
                ),
                23 => openttd_rust_company_decrease_loan(
                    &FINANCE,
                    variant,
                    self.amount,
                    variant == 3,
                    self.execute,
                ),
                24 => openttd_rust_company_set_max_loan(&FINANCE, id, self.amount, self.execute),
                25 => openttd_rust_company_change_bank_balance(
                    &FINANCE,
                    u32::from(variant % 2),
                    self.amount,
                    id,
                    if variant == 14 { 0 } else { variant },
                    self.execute,
                ),
                _ => unreachable!(),
            }
        }
    }

    fn observation(result: Cost) -> Vec<i64> {
        FIXTURE.with_borrow(|f| {
            let mut values = vec![
                i64::from(result.error),
                result.cost,
                i64::from(result.expense),
                result.param,
                i64::from(f.current),
            ];
            for &owner in &f.owners {
                // SAFETY: All synchronous production calls have returned. This
                // test exclusively owns all three live canonical allocations.
                let c = unsafe { &*owner };
                values.extend([
                    c.money,
                    c.loan,
                    c.max_loan,
                    c.current.income,
                    c.current.expenses,
                ]);
                values.extend(c.expenses.iter().flatten());
            }
            values.push(f.trace.len() as i64);
            values.extend(f.trace.iter().flatten());
            values
        })
    }

    /// Observation field ranges of the oracle's sections, in observation order.
    const SECTIONS: [(&str, usize, Option<usize>); 7] = [
        ("result", 0, Some(5)),
        ("company0", 5, Some(49)),
        ("company1-money", 49, Some(50)),
        ("company1-loan", 50, Some(52)),
        ("company1-finances", 52, Some(93)),
        ("company2", 93, Some(137)),
        ("trace", 137, None),
    ];

    #[test]
    fn all_financial_commands_match_original() {
        let _guard = ECONOMY_LOCK.lock().unwrap();
        let _run = Run::start();
        let mut actual: Vec<Vec<(String, String)>> = vec![Vec::new(); SECTIONS.len()];
        for row in 0..2_520 {
            let case = Case::from_index(row);
            for variant in VARIANTS {
                case.reset(variant);
                let values = observation(case.invoke(variant));
                let inputs = format!(
                    "op {} execute {} money {} loan {} amount {} variant {variant}",
                    case.op, case.execute, case.money, case.loan, case.amount
                );
                for (stream, &(_, start, end)) in actual.iter_mut().zip(&SECTIONS) {
                    let fields = &values[start..end.unwrap_or(values.len())];
                    let text: Vec<String> = fields.iter().map(i64::to_string).collect();
                    stream.push((inputs.clone(), text.join(" ")));
                }
            }
        }
        let oracle = crate::oracle::parse(include_str!("company/financial-command-oracle.txt"));
        assert_eq!(oracle.len(), SECTIONS.len());
        for (stream, &(name, _, _)) in actual.into_iter().zip(&SECTIONS) {
            assert_eq!(crate::oracle::check(&oracle, name, stream), 40_320);
        }
    }
}

/// Road build, clear and convert price saturation, rounding and independent type masks, with
/// unclassified `NewGRF` masks and extreme prices that the default maps never load.
#[cfg(test)]
mod road_price_tests {
    use crate::economy_type::Price;
    use crate::road_cost::{build_cost, clear_cost, convert_cost};
    use crate::road_type::{ROADTYPE_ROAD, ROADTYPE_TRAM};

    const PR_BUILD_ROAD: Price = Price::PR_BUILD_ROAD;
    const PR_CLEAR_ROAD: Price = Price::PR_CLEAR_ROAD;

    extern "C" fn cost_multiplier(road_type: u8) -> u16 {
        match road_type {
            ROADTYPE_ROAD => 3,
            ROADTYPE_TRAM => u16::MAX,
            _ => unreachable!(),
        }
    }
    fn set_price(index: usize, value: i64) {
        // SAFETY: Process-wide price table, serialized by the game-state test lock.
        unsafe {
            crate::company::openttd_rust_economy_prices()
                .add(index)
                .write(value);
        }
    }

    #[test]
    fn road_price_saturation_rounding_and_type_masks() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let prices = crate::company::openttd_rust_economy_prices();
        let (road_mask, tram_mask) = (
            crate::type_registry::openttd_rust_road_mask(),
            crate::type_registry::openttd_rust_tram_mask(),
        );
        // SAFETY: Process-wide scalars, serialized by the lock and restored below.
        let saved = unsafe {
            (
                prices.add(PR_BUILD_ROAD.0 as usize).read(),
                prices.add(PR_CLEAR_ROAD.0 as usize).read(),
                road_mask.replace(1 << ROADTYPE_ROAD),
                tram_mask.replace(1 << ROADTYPE_TRAM),
            )
        };
        let mut s = crate::test_tables::SHARED;
        s.road_cost_multiplier = cost_multiplier;
        set_price(PR_BUILD_ROAD.0 as usize, 7);
        set_price(PR_CLEAR_ROAD.0 as usize, -27);
        assert_eq!(build_cost(&s, ROADTYPE_ROAD), 2);
        assert_eq!(clear_cost(&s, ROADTYPE_ROAD), -27);
        assert_eq!(convert_cost(&s, ROADTYPE_ROAD, ROADTYPE_ROAD), 0);
        // Neither road nor tram: the source clear policy still earns removal value.
        // SAFETY: As above.
        unsafe { road_mask.write(0) };
        assert_eq!(clear_cost(&s, ROADTYPE_ROAD), -28);
        set_price(PR_BUILD_ROAD.0 as usize, -7);
        assert_eq!(build_cost(&s, ROADTYPE_ROAD), -3);
        assert_eq!(clear_cost(&s, ROADTYPE_ROAD), -25);
        set_price(PR_BUILD_ROAD.0 as usize, i64::MAX);
        assert_eq!(build_cost(&s, ROADTYPE_TRAM), 1_152_921_504_606_846_975);
        assert_eq!(clear_cost(&s, ROADTYPE_TRAM), -864_691_128_455_135_258);
        set_price(PR_BUILD_ROAD.0 as usize, i64::MIN);
        assert_eq!(build_cost(&s, ROADTYPE_TRAM), -1_152_921_504_606_846_976);
        assert_eq!(clear_cost(&s, ROADTYPE_TRAM), 864_691_128_455_135_205);
        // SAFETY: As above.
        unsafe {
            prices.add(PR_BUILD_ROAD.0 as usize).write(saved.0);
            prices.add(PR_CLEAR_ROAD.0 as usize).write(saved.1);
            road_mask.write(saved.2);
            tram_mask.write(saved.3);
        }
    }
}

/// Company control: the first playable company and its allocation check, service intervals,
/// ownership errors with both original assertions before error selection, and the
/// special-owner takeover mask.
#[cfg(test)]
mod control_tests {
    use super::*;
    use crate::company_type::{COMPANY_SPECTATOR, CompanyID, OWNER_END, OpenTTDCompany, Owner};
    use crate::settings_table::VehicleDefaults;
    use crate::test_tables::{COMMAND_COSTS, COMPANY, COMPANY_IDENTITY, FINANCE, MOVEMENT, SHARED};
    use crate::tile_type::TileIndex;
    use std::cell::{Cell, RefCell};

    /// Per company slot: `None` free, else whether it is an AI.
    type Slots = [Option<bool>; MAX_COMPANIES as usize];
    thread_local! {
        static SLOTS: RefCell<Slots> = const { RefCell::new([None; MAX_COMPANIES as usize]) };
        static CURRENT: Cell<u8> = const { Cell::new(0) };
        static LOCAL: Cell<u8> = const { Cell::new(0) };
        static ERRORS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
        static FAILED: RefCell<Vec<bool>> = const { RefCell::new(Vec::new()) };
        /// Calls to `Company::CanAllocateItem`, each of which grants the native pool's
        /// allocation credit that the caller's next allocation consumes.
        static ALLOCATION_CHECKS: Cell<u32> = const { Cell::new(0) };
        /// What the next `Company::CanAllocateItem` returns, when set.
        static ALLOCATION_ANSWER: Cell<Option<bool>> = const { Cell::new(None) };
    }
    fn handle(id: CompanyID) -> *mut OpenTTDCompany {
        std::ptr::without_provenance_mut(usize::from(id.0) + 1)
    }
    extern "C" fn company(id: CompanyID) -> *mut OpenTTDCompany {
        handle(id)
    }
    extern "C" fn company_id(c: *mut OpenTTDCompany) -> CompanyID {
        CompanyID((c.addr() - 1) as u8)
    }
    extern "C" fn next_company(from: u32) -> *mut OpenTTDCompany {
        SLOTS.with(|s| {
            let s = s.borrow();
            (from..u32::from(MAX_COMPANIES))
                .find(|&id| s[id as usize].is_some())
                .map_or(std::ptr::null_mut(), |id| handle(CompanyID(id as u8)))
        })
    }
    extern "C" fn is_ai(c: *mut OpenTTDCompany) -> bool {
        SLOTS.with(|s| s.borrow()[usize::from(company_id(c).0)].unwrap())
    }
    extern "C" fn company_valid(id: u8) -> bool {
        SLOTS.with(|s| s.borrow().get(usize::from(id)).copied().flatten().is_some())
    }
    extern "C" fn can_allocate() -> bool {
        ALLOCATION_CHECKS.set(ALLOCATION_CHECKS.get() + 1);
        ALLOCATION_ANSWER
            .get()
            .unwrap_or_else(|| SLOTS.with(|s| s.borrow().iter().any(Option::is_none)))
    }
    extern "C" fn current_company() -> u8 {
        CURRENT.get()
    }
    extern "C" fn local_company() -> u8 {
        LOCAL.get()
    }
    fn log(entry: String) {
        ERRORS.with(|e| e.borrow_mut().push(entry));
    }
    extern "C" fn zero(_: *mut crate::services::CommandCost) {
        log("zero".to_owned());
    }
    extern "C" fn error(_: *mut crate::services::CommandCost, message: u32) {
        log(format!("error {message}"));
    }
    extern "C" fn encode_owned_by(
        _: *mut crate::services::CommandCost,
        owner: Owner,
        tile: TileIndex,
    ) {
        log(format!("encode {} {}", owner.0, tile.0));
    }
    extern "C" fn error_owner(_: *mut crate::services::CommandCost, owner: Owner) {
        log(format!("owner {}", owner.0));
    }
    extern "C" fn native_assert(condition: bool) {
        FAILED.with(|f| f.borrow_mut().push(condition));
    }

    const OWNED_BY: u32 = 0x7001;
    fn leak<T>(value: T) -> &'static T {
        Box::leak(Box::new(value))
    }
    fn identity() -> crate::company_identity::Services {
        let mut identity = COMPANY_IDENTITY;
        identity.company = company;
        identity.company_id = company_id;
        identity.next_company = next_company;
        identity.is_ai = is_ai;
        identity
    }
    fn finance() -> FinanceServices {
        let mut finance = FINANCE;
        finance.current_company = current_company;
        finance
    }
    fn services() -> Services {
        let identity = identity();
        let mut movement = MOVEMENT;
        movement.native_assert = native_assert;
        let mut shared = SHARED;
        shared.company_valid = company_valid;
        shared.movement = leak(movement);
        let finance = finance();
        let mut costs = COMMAND_COSTS;
        costs.zero = zero;
        costs.error = error;
        costs.encode_owned_by = encode_owned_by;
        costs.error_owner = error_owner;
        let mut s = COMPANY;
        s.identity = leak(identity);
        s.shared = leak(shared);
        s.finance = leak(finance);
        s.cost = leak(costs);
        s.can_allocate_company = can_allocate;
        s.local_company = local_company;
        s.owned_by = OWNED_BY;
        s
    }
    fn set(id: CompanyID, slot: Option<bool>) {
        SLOTS.with(|s| s.borrow_mut()[usize::from(id.0)] = slot);
    }
    fn check(s: &Services, owner: Owner, tile: TileIndex) -> Vec<String> {
        ERRORS.with(|e| e.borrow_mut().clear());
        check_ownership(
            s,
            crate::services::Cost(std::ptr::null_mut(), s.cost),
            owner,
            tile,
        );
        ERRORS.with(|e| e.borrow().clone())
    }

    thread_local! {
        static NETWORKING: Cell<bool> = const { Cell::new(false) };
        static OFFERED: Cell<*mut Finances> = const { Cell::new(std::ptr::null_mut()) };
    }
    extern "C" fn networking() -> bool {
        NETWORKING.get()
    }
    extern "C" fn offered_finances(id: u8) -> *mut Finances {
        if id == 1 {
            OFFERED.get()
        } else {
            std::ptr::null_mut()
        }
    }
    extern "C" fn human(_: *mut OpenTTDCompany) -> bool {
        false
    }

    /// "Company economy - special owners have no bankruptcy takeover offer": no current owner
    /// at or above 16, which lies outside the 16-bit offer mask, can buy company 1, hostile or
    /// not, whether no offer or every offer bit is set.
    #[test]
    fn special_owners_have_no_takeover_offer() {
        let mut s = services();
        let mut finance = finance();
        finance.networking = networking;
        finance.company_finances = offered_finances;
        s.finance = leak(finance);
        let mut identity = identity();
        identity.is_ai = human;
        s.identity = leak(identity);
        NETWORKING.set(true);
        // SAFETY: Plain integer record.
        let mut offered: Box<Finances> = Box::new(unsafe { std::mem::zeroed() });
        OFFERED.set(&raw mut *offered);
        let ownership = crate::tile_owner::Context {
            services: std::ptr::null(),
            cost: std::ptr::null_mut(),
            ret: std::ptr::null_mut(),
            error: std::ptr::null_mut(),
            road: std::ptr::null(),
        };
        for asked in [0, u16::MAX] {
            offered.asked = asked;
            for current in 16..=u8::MAX {
                CURRENT.set(current);
                for hostile in [false, true] {
                    let result = buy(&s, &ownership, 1, hostile, false);
                    assert_eq!(
                        result.error, 1,
                        "asked {asked:#x} current {current} hostile {hostile}"
                    );
                }
            }
        }
        OFFERED.set(std::ptr::null_mut());
    }

    /// The first existing human company wins, then the first free slot, then the first ID.
    #[test]
    fn first_playable_prefers_humans_then_free_slots() {
        let s = services();
        SLOTS.with(|slots| *slots.borrow_mut() = [None; MAX_COMPANIES as usize]);
        ALLOCATION_ANSWER.set(None);
        // Each selection, with the number of `Company::CanAllocateItem` calls it makes: a
        // human returns before the check; otherwise the check runs once and grants the
        // allocation credit that the caller's `new (vacant) Company` consumes.
        let select = |checks: u32| {
            ALLOCATION_CHECKS.set(0);
            let id = first_playable(&s).0;
            assert_eq!(
                ALLOCATION_CHECKS.get(),
                checks,
                "allocation checks for {id}"
            );
            id
        };
        set(CompanyID(3), Some(false));
        assert_eq!(select(0), 3, "a human wins before the hole at 0");
        set(CompanyID(3), Some(true));
        assert_eq!(select(1), 0);
        for id in (0..MAX_COMPANIES).filter(|&id| id != 3) {
            set(CompanyID(id), Some(true));
        }
        assert_eq!(select(1), CompanyID::BEGIN.0, "a full AI pool");
        set(CompanyID(9), Some(false));
        set(CompanyID(12), Some(false));
        assert_eq!(select(0), 9);
        set(CompanyID(1), None);
        assert_eq!(select(0), 9);
        set(CompanyID(9), Some(true));
        set(CompanyID(12), Some(true));
        assert_eq!(select(1), 1);
        // The free slot is taken only when the pool grants the allocation.
        ALLOCATION_ANSWER.set(Some(false));
        assert_eq!(select(1), CompanyID::BEGIN.0, "a refused allocation");
        ALLOCATION_ANSWER.set(None);
    }

    /// Each vehicle type reads its own interval field, ignoring the percent flag.
    #[test]
    fn service_intervals_read_the_type_field() {
        let s = services();
        for percent in [false, true] {
            let company = VehicleDefaults {
                servint_ispercent: percent,
                servint_trains: 0,
                servint_roadveh: 121,
                servint_aircraft: 65535,
                servint_ships: 423,
            };
            let client = VehicleDefaults {
                servint_ispercent: !percent,
                servint_trains: 224,
                servint_roadveh: 0,
                servint_aircraft: 426,
                servint_ships: 65535,
            };
            for (kind, company_interval, client_interval) in [
                (crate::vehicle_type::VehicleType::VEH_TRAIN, 0, 224),
                (crate::vehicle_type::VehicleType::VEH_ROAD, 121, 0),
                (crate::vehicle_type::VehicleType::VEH_AIRCRAFT, 65535, 426),
                (crate::vehicle_type::VehicleType::VEH_SHIP, 423, 65535),
            ] {
                assert_eq!(
                    service_interval_from(s.shared, &raw const company, kind),
                    company_interval
                );
                assert_eq!(
                    service_interval_from(s.shared, &raw const client, kind),
                    client_interval
                );
            }
        }
    }

    /// The current company owns; another owner fails with the owned-by error, encoded with its
    /// owner only for the local company.
    #[test]
    fn ownership_errors_and_assertions() {
        let s = services();
        CURRENT.set(0);
        for observer in [0, COMPANY_SPECTATOR] {
            LOCAL.set(observer);
            assert_eq!(check(&s, CompanyID(0), TileIndex(0)), ["zero"]);
            for owner in [CompanyID(3), Owner::NONE] {
                let mut expected = vec![format!("error {OWNED_BY}")];
                if observer == 0 {
                    expected.extend([
                        format!("encode {} 0", owner.0),
                        format!("owner {}", owner.0),
                    ]);
                }
                assert_eq!(
                    check(&s, owner, TileIndex(0)),
                    expected,
                    "observer {observer} owner {}",
                    owner.0
                );
            }
        }
        // Both original assertions precede error selection; a spectator gets no encoding.
        LOCAL.set(COMPANY_SPECTATOR);
        for (owner, tile) in [
            (CompanyID(OWNER_END), TileIndex(1)),
            (Owner::TOWN, TileIndex(0)),
        ] {
            FAILED.with(|f| f.borrow_mut().clear());
            assert_eq!(check(&s, owner, tile), [format!("error {OWNED_BY}")]);
            assert_eq!(
                FAILED.with(|f| f.borrow().clone()),
                [false],
                "owner {}",
                owner.0
            );
        }
    }
}

/// Competitor posts return to the caller before each post: `on_tick`'s return protocol, driven
/// by a loop of the session caller's form (`while on_tick(..) { post_ai }`).
#[cfg(test)]
mod competitor_tests {
    use super::*;
    use crate::openttd::GameMode;
    use crate::test_tables::{COMPANY, FINANCE, SHARED};
    use std::cell::{Cell, RefCell};

    thread_local! {
        static TRACE: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
        static MAX_COMPETITORS: Cell<u8> = const { Cell::new(0) };
        static COMPANY_LIMIT: Cell<u8> = const { Cell::new(15) };
        static INTERVAL: Cell<u16> = const { Cell::new(0) };
    }
    extern "C" fn yes() -> bool {
        true
    }
    extern "C" fn no() -> bool {
        false
    }
    extern "C" fn none(_: u8) -> *mut Finances {
        std::ptr::null_mut()
    }
    extern "C" fn no_companies() -> usize {
        0
    }
    extern "C" fn end(_: u32) -> CompanyRef {
        CompanyRef {
            finances: std::ptr::null_mut(),
            id: INVALID,
        }
    }
    extern "C" fn no_random(_: u32) -> u32 {
        0
    }
    extern "C" fn reset_timeout(ticks: u32) {
        TRACE.with(|t| t.borrow_mut().push(format!("timeout {ticks}")));
    }
    fn leak<T>(value: T) -> &'static T {
        Box::leak(Box::new(value))
    }
    fn services() -> Services {
        let mut shared = SHARED;
        shared.settings = leak(crate::settings_type::SettingAddresses {
            max_competitors: MAX_COMPETITORS.with(Cell::as_ptr),
            max_companies: COMPANY_LIMIT.with(Cell::as_ptr),
            competitor_interval: INTERVAL.with(Cell::as_ptr),
            ..crate::settings_type::SettingAddresses::NULL
        });
        let mut finance = FINANCE;
        finance.networking = no;
        finance.company_finances = none;
        let mut s = COMPANY;
        s.shared = leak(shared);
        s.finance = leak(finance);
        s.competitor_fired = yes;
        s.ai_can_start = yes;
        s.company_count = no_companies;
        s.company_next = end;
        s.script_random_next = no_random;
        s.reset_competitor_timeout = reset_timeout;
        s
    }
    fn set_mode(mode: GameMode) -> GameMode {
        // SAFETY: The canonical session state, serialized by the game-state test lock.
        unsafe {
            let state = crate::session::openttd_rust_session_storage();
            std::mem::replace(&mut (*state).mode, mode)
        }
    }

    #[test]
    fn competitor_posts_return_before_each_post() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let s = services();
        let saved_mode = set_mode(GameMode::NORMAL);
        for change_limit in [false, true] {
            TRACE.with(|t| t.borrow_mut().clear());
            // SAFETY: The process-wide tick cursor, serialized by the lock.
            let cursor = unsafe { *openttd_rust_company_tick() };
            MAX_COMPETITORS.set(2);
            INTERVAL.set(0);
            let mut tick = Tick::default();
            let mut posts = 0;
            // SAFETY: The test's live services; posts run between calls, as natively.
            while unsafe { on_tick(&s, &mut tick) } {
                assert!(
                    TRACE.with(|t| t.borrow().is_empty()),
                    "the caller posts before the timeout"
                );
                posts += 1;
                if change_limit {
                    MAX_COMPETITORS.set(1);
                }
            }
            assert_eq!(posts, if change_limit { 1 } else { 2 });
            // The competitor timeout reads the same settings and game mode in place.
            INTERVAL.set(1);
            MAX_COMPETITORS.set(1);
            assert_eq!(openttd_rust_company_competitor_timeout_abi(&s), 1);
            MAX_COMPETITORS.set(0);
            assert_eq!(openttd_rust_company_competitor_timeout_abi(&s), 0);
            MAX_COMPETITORS.set(1);
            INTERVAL.set(0);
            assert_eq!(openttd_rust_company_competitor_timeout_abi(&s), 0);
            assert_eq!(TRACE.with(|t| t.borrow().clone()), ["timeout 19425"]);
            // SAFETY: As above.
            assert_eq!(
                unsafe { *openttd_rust_company_tick() },
                (cursor + 1) % u32::from(MAX_COMPANIES)
            );
        }
        set_mode(saved_mode);
    }
}

/// Company startup and control continuations: the AI startup continuation and abandoned
/// frames, and a network client's local switch before the settings continuation. Company 2 is
/// the slot every allocation takes.
#[cfg(test)]
mod startup_tests {
    use super::*;
    use crate::company_type::{COMPANY_SPECTATOR, CompanyID, CompanyManagerFace, OpenTTDCompany};
    use crate::livery::{Livery, LiveryScheme};
    use crate::test_tables::{
        COMMAND_COSTS, COMMAND_POST, COMPANY, COMPANY_IDENTITY, FACES, FINANCE, FLEET_GROUPS,
        MOVEMENT, NETWORK_COMMAND, NETWORK_FRAME, OBJECTIVES, SHARED,
    };
    use std::cell::{Cell, RefCell};

    const ID: u8 = 2;
    /// Company 2's native record: colour, liveries, face bits and president.
    struct Record {
        colour: u8,
        liveries: [Livery; LiveryScheme::LS_END.0 as usize],
        face_bits: u32,
        president: (u32, u32),
    }
    thread_local! {
        static TRACE: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
        static RECORD: RefCell<Record> = RefCell::new(Record {
            colour: 0,
            liveries: [Livery::default(); LiveryScheme::LS_END.0 as usize],
            face_bits: 0,
            president: (0, 0),
        });
        static FINANCES: Cell<*mut Finances> = const { Cell::new(std::ptr::null_mut()) };
        static NETWORKING: Cell<bool> = const { Cell::new(false) };
        static SERVER: Cell<bool> = const { Cell::new(false) };
        static CURRENT: Cell<u8> = const { Cell::new(18) };
        static LOCAL: Cell<u8> = const { Cell::new(0) };
        static OWN: Cell<bool> = const { Cell::new(false) };
        static POST: Cell<*const crate::command_post::Services> = const { Cell::new(std::ptr::null()) };
        static NETWORK: Cell<*const crate::network_command::Services> = const { Cell::new(std::ptr::null()) };
        static FRAME: Cell<*const crate::network_frame::Services> = const { Cell::new(std::ptr::null()) };
        static WINDOWS: Cell<*const crate::objectives::Services> = const { Cell::new(std::ptr::null()) };
    }
    fn trace(entry: &'static str) {
        TRACE.with(|t| t.borrow_mut().push(entry));
    }
    fn traced() -> Vec<&'static str> {
        TRACE.with(|t| t.borrow().clone())
    }
    fn handle() -> *mut OpenTTDCompany {
        std::ptr::without_provenance_mut(usize::from(ID) + 1)
    }
    extern "C" fn company(_: CompanyID) -> *mut OpenTTDCompany {
        handle()
    }
    extern "C" fn company_id(_: *mut OpenTTDCompany) -> CompanyID {
        CompanyID(ID)
    }
    extern "C" fn no_company(_: u32) -> *mut OpenTTDCompany {
        std::ptr::null_mut()
    }
    extern "C" fn colour(_: *mut OpenTTDCompany) -> u8 {
        RECORD.with(|r| r.borrow().colour)
    }
    extern "C" fn set_colour(_: *mut OpenTTDCompany, value: u8) {
        RECORD.with(|r| r.borrow_mut().colour = value);
    }
    extern "C" fn livery(_: *mut OpenTTDCompany, scheme: u8) -> *mut Livery {
        RECORD.with(|r| &raw mut r.borrow_mut().liveries[usize::from(scheme)])
    }
    extern "C" fn face(_: *mut OpenTTDCompany) -> *mut CompanyManagerFace {
        std::ptr::without_provenance_mut(1)
    }
    extern "C" fn face_bits(_: *mut CompanyManagerFace) -> *mut u32 {
        RECORD.with(|r| &raw mut r.borrow_mut().face_bits)
    }
    extern "C" fn set_president_id(_: *mut OpenTTDCompany, value: u32) {
        RECORD.with(|r| r.borrow_mut().president.0 = value);
    }
    extern "C" fn set_president_parts(_: *mut OpenTTDCompany, value: u32) {
        RECORD.with(|r| r.borrow_mut().president.1 = value);
    }
    extern "C" fn not_ai(_: *mut OpenTTDCompany) -> bool {
        false
    }
    extern "C" fn name_frame(_: *mut crate::company_identity::NameStorage) {}
    extern "C" fn render(
        _: *mut crate::company_identity::NameStorage,
        _: u32,
        _: u32,
    ) -> crate::services::ByteSpan {
        crate::services::ByteSpan {
            data: b"President".as_ptr(),
            size: 9,
        }
    }
    extern "C" fn utf8_length(_: *const u8, size: usize) -> usize {
        size
    }
    extern "C" fn no_draw(_: *mut std::ffi::c_void) -> u32 {
        0
    }
    extern "C" fn one_style() -> u32 {
        1
    }
    extern "C" fn face_random(_: *mut crate::town::names::Randomizer) -> u32 {
        0
    }
    extern "C" fn no_vars(_: u32) -> crate::company_manager_face::FaceVars {
        crate::company_manager_face::FaceVars {
            data: std::ptr::null(),
            size: 0,
        }
    }
    extern "C" fn face_spec(_: u32) -> *const crate::company_manager_face::FaceSpec {
        std::ptr::without_provenance(1)
    }
    extern "C" fn set_style(_: *mut CompanyManagerFace, _: u32) {}
    extern "C" fn set_label(
        _: *mut CompanyManagerFace,
        _: *const crate::company_manager_face::FaceSpec,
    ) {
    }
    extern "C" fn style(_: *mut CompanyManagerFace) -> u32 {
        0
    }
    extern "C" fn no_group(_: u32) -> u32 {
        u32::MAX
    }
    extern "C" fn admin_update(_: CompanyID) {
        trace("admin");
    }
    extern "C" fn reset_colourmap() {
        trace("colourmap");
    }
    extern "C" fn screen_dirty() {
        trace("screen");
    }
    extern "C" fn company_valid(id: u8) -> bool {
        id == ID && !FINANCES.get().is_null()
    }
    extern "C" fn holds(condition: bool) {
        assert!(condition);
    }
    extern "C" fn command_post() -> &'static crate::command_post::Services {
        // SAFETY: Leaked by `services` for the test process.
        unsafe { &*POST.get() }
    }
    extern "C" fn network_command() -> &'static crate::network_command::Services {
        // SAFETY: As above.
        unsafe { &*NETWORK.get() }
    }
    extern "C" fn network_frame() -> &'static crate::network_frame::Services {
        // SAFETY: As above.
        unsafe { &*FRAME.get() }
    }
    extern "C" fn objectives() -> &'static crate::objectives::Services {
        // SAFETY: As above.
        unsafe { &*WINDOWS.get() }
    }
    extern "C" fn no_client(_: u32) -> *mut std::ffi::c_void {
        std::ptr::null_mut()
    }

    extern "C" fn networking() -> bool {
        NETWORKING.get()
    }
    extern "C" fn server() -> bool {
        SERVER.get()
    }
    extern "C" fn not_server() -> bool {
        false
    }
    extern "C" fn current() -> u8 {
        CURRENT.get()
    }
    extern "C" fn set_current(id: u8) {
        CURRENT.set(id);
    }
    extern "C" fn local() -> u8 {
        LOCAL.get()
    }
    extern "C" fn finances(id: u8) -> *mut Finances {
        if id == ID {
            FINANCES.get()
        } else {
            std::ptr::null_mut()
        }
    }
    extern "C" fn yes() -> bool {
        true
    }
    extern "C" fn allocate(_: u8, _: bool) -> CompanyRef {
        // SAFETY: Plain integer record, freed by `reset`.
        let finances = Box::into_raw(Box::new(unsafe { std::mem::zeroed::<Finances>() }));
        FINANCES.set(finances);
        trace("allocate 2");
        CompanyRef {
            finances,
            id: u32::from(ID),
        }
    }
    extern "C" fn prepare(_: u8, _: bool) -> bool {
        trace("setup 2");
        true
    }
    extern "C" fn events(_: u8) {
        trace("events 2");
    }
    extern "C" fn network_new(_: u8) {
        trace("network 2");
    }
    extern "C" fn ignore(_: u8) {}
    extern "C" fn nothing() {}
    extern "C" fn own_company(_: u8, client: u32) -> *mut OpenTTDCompany {
        assert_eq!(client, 7);
        trace("request");
        if OWN.get() {
            handle()
        } else {
            std::ptr::null_mut()
        }
    }
    extern "C" fn own_settings(_: *mut OpenTTDCompany) {
        assert_eq!((LOCAL.get(), CURRENT.get()), (ID, ID));
        trace("settings");
    }
    extern "C" fn window_data(class: u16, number: u32, data: i32) {
        assert_eq!(
            (class, number, data),
            (
                crate::window_type::WC_SEND_NETWORK_MSG,
                crate::network_type::DESTTYPE_TEAM,
                i32::from(COMPANY_SPECTATOR)
            )
        );
        assert_eq!(
            (LOCAL.get(), CURRENT.get()),
            (COMPANY_SPECTATOR, COMPANY_SPECTATOR)
        );
        trace("chat");
    }
    extern "C" fn window_classes(_: u16, _: i32) {
        trace("classes");
    }
    extern "C" fn assign(company: CompanyID) {
        LOCAL.set(company.0);
        CURRENT.set(company.0);
        trace("assign");
    }
    extern "C" fn close_construction() {
        trace("construction");
    }

    fn leak<T>(value: T) -> &'static T {
        Box::leak(Box::new(value))
    }
    fn shared() -> &'static crate::services::Services {
        let mut movement = MOVEMENT;
        movement.native_assert = holds;
        let mut shared = SHARED;
        shared.random = no_draw;
        shared.utf8_length = utf8_length;
        shared.screen_dirty = screen_dirty;
        shared.company_valid = company_valid;
        shared.current_company = current;
        shared.command_post = command_post;
        shared.network_client_info = no_client;
        shared.movement = leak(movement);
        leak(shared)
    }
    fn identity(
        shared: &'static crate::services::Services,
    ) -> &'static crate::company_identity::Services {
        let mut faces = FACES;
        faces.count = one_style;
        faces.random = face_random;
        faces.vars = no_vars;
        faces.spec = face_spec;
        faces.set_style = set_style;
        faces.set_label = set_label;
        faces.style = style;
        faces.bits = face_bits;
        let mut groups = FLEET_GROUPS;
        groups.next_group = no_group;
        let mut identity = COMPANY_IDENTITY;
        identity.shared = shared;
        identity.faces = leak(faces);
        identity.groups = leak(groups);
        identity.company = company;
        identity.next_company = no_company;
        identity.company_id = company_id;
        identity.colour = colour;
        identity.set_colour = set_colour;
        identity.livery = livery;
        identity.face = face;
        identity.set_president_id = set_president_id;
        identity.set_president_parts = set_president_parts;
        identity.is_ai = not_ai;
        identity.name_frame_init = name_frame;
        identity.name_frame_drop = name_frame;
        identity.render = render;
        identity.admin_update = admin_update;
        identity.reset_colourmap = reset_colourmap;
        leak(identity)
    }
    /// The fixture's `FixtureReset`: company services over company 2's record, with the
    /// network leg (`network_new`) seeing a non-server company table.
    fn services(network: bool, is_server: bool) -> &'static Services {
        reset();
        NETWORKING.set(network);
        SERVER.set(is_server);
        let shared = shared();
        let mut finance = FINANCE;
        finance.networking = networking;
        finance.current_company = current;
        finance.set_current_company = set_current;
        finance.company_finances = finances;
        let mut windows = OBJECTIVES;
        windows.window_data = window_data;
        windows.window_classes = window_classes;
        WINDOWS.set(leak(windows));
        let mut s = COMPANY;
        s.identity = identity(shared);
        s.shared = shared;
        s.finance = leak(finance);
        s.cost = &COMMAND_COSTS;
        s.objectives = objectives;
        s.local_company = local;
        s.network_server = server;
        s.can_allocate_company = yes;
        s.allocate_company = allocate;
        s.prepare_new_company = prepare;
        s.company_defaults = ignore;
        s.startup_windows = nothing;
        s.new_company_events = events;
        s.company_league_dirty = nothing;
        s.company_ctrl_windows = nothing;
        s.assert_new_ai_slot = ignore;
        s.admin_company_new = network_new;
        s.close_network_status = nothing;
        s.network_own_company = own_company;
        s.network_own_company_settings = own_settings;
        s.assign_company_context = assign;
        s.close_construction_windows = close_construction;
        let s = leak(s);
        // `network_new` reads the network command's own company table, which is no server.
        let mut network_company = COMPANY;
        network_company.shared = shared;
        network_company.identity = s.identity;
        network_company.network_server = not_server;
        let mut command = NETWORK_COMMAND;
        command.company = leak(network_company);
        command.network = network_frame;
        let command = leak(command);
        NETWORK.set(command);
        let mut frame = NETWORK_FRAME;
        frame.command = command;
        FRAME.set(leak(frame));
        let mut post = COMMAND_POST;
        post.network = network_command;
        POST.set(leak(post));
        s
    }
    fn reset() {
        TRACE.with(|t| t.borrow_mut().clear());
        CURRENT.set(18);
        LOCAL.set(0);
        let finances = FINANCES.replace(std::ptr::null_mut());
        if !finances.is_null() {
            // SAFETY: Allocated by `allocate`.
            drop(unsafe { Box::from_raw(finances) });
        }
    }
    fn ownership() -> crate::tile_owner::Context {
        crate::tile_owner::Context {
            services: std::ptr::null(),
            cost: std::ptr::null_mut(),
            ret: std::ptr::null_mut(),
            error: std::ptr::null_mut(),
            road: std::ptr::null(),
        }
    }
    fn control_frame(action: u8, client: crate::network_type::ClientID) -> Control {
        Control {
            result: Cost {
                cost: 0,
                param: 0,
                error: 0,
                expense: 0,
            },
            startup: Startup {
                company: 0,
                requested: 0,
                is_ai: false,
                stage: 0,
            },
            client: client.0,
            action,
            target: INVALID_OWNER,
            reason: 0,
            execute: true,
            stage: 0,
        }
    }
    fn colour_scope<R>(f: impl FnOnce() -> R) -> R {
        // SAFETY: The process-wide colour cache, serialized by the game-state test lock.
        let slot =
            unsafe { crate::company_identity::openttd_rust_company_colours().add(usize::from(ID)) };
        // SAFETY: As above.
        let saved = unsafe { slot.read() };
        let result = f();
        // SAFETY: As above.
        unsafe { slot.write(saved) };
        result
    }

    /// Rust returns before `AI::StartNew`; an abandoned frame keeps the allocated company
    /// without new-company events or a network announcement, a resumed one completes once,
    /// and a network client starts no AI.
    #[test]
    fn ai_startup_continuation_and_abandoned_frames() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        // Startup reads the economy's prices and maximum loan.
        let _economy = super::financial_command_tests::ECONOMY_LOCK.lock().unwrap();
        colour_scope(|| {
            let ownership = ownership();
            let s = services(false, false);
            let mut abandoned = control_frame(
                crate::company_type::CCA_NEW_AI,
                crate::network_type::ClientID::INVALID_CLIENT_ID,
            );
            assert!(control(s, &ownership, &mut abandoned));
            assert_eq!(abandoned.startup.company, u32::from(ID));
            assert_eq!(traced(), ["allocate 2", "setup 2"]);
            // SAFETY: Company 2's live finances.
            let (money, loan) = unsafe { ((*FINANCES.get()).money, (*FINANCES.get()).loan) };
            assert_eq!(money, loan);

            let s = services(false, false);
            let mut resumed = control_frame(
                crate::company_type::CCA_NEW_AI,
                crate::network_type::ClientID::INVALID_CLIENT_ID,
            );
            let mut starts = 0;
            while control(s, &ownership, &mut resumed) {
                starts += 1;
            }
            assert_eq!(starts, 1);
            assert_eq!(traced(), ["allocate 2", "setup 2", "events 2", "network 2"]);
            assert_eq!(resumed.result.error, 0);

            let s = services(true, false);
            let mut client = Startup {
                company: u32::MAX,
                requested: INVALID_OWNER,
                is_ai: true,
                stage: 0,
            };
            assert!(!startup(s, &mut client));
            assert_eq!(traced(), ["allocate 2", "setup 2", "events 2"]);
            reset();
        });
    }

    /// A network client's new company switches the local company before the native settings
    /// continuation, and only when the client owns it.
    #[test]
    fn network_local_switch_precedes_settings_continuation() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        // Startup reads the economy's prices and maximum loan.
        let _economy = super::financial_command_tests::ECONOMY_LOCK.lock().unwrap();
        colour_scope(|| {
            let ownership = ownership();
            for matching_client in [false, true] {
                let s = services(true, false);
                LOCAL.set(COMPANY_SPECTATOR);
                CURRENT.set(COMPANY_SPECTATOR);
                OWN.set(matching_client);
                let mut call = control_frame(
                    crate::company_type::CCA_NEW,
                    crate::network_type::ClientID(7),
                );
                assert!(!control(s, &ownership, &mut call));
                let mut expected =
                    vec!["allocate 2", "setup 2", "events 2", "network 2", "request"];
                if matching_client {
                    expected.extend([
                        "chat",
                        "assign",
                        "classes",
                        "construction",
                        "screen",
                        "classes",
                        "classes",
                        "classes",
                        "colourmap",
                        "settings",
                    ]);
                }
                assert_eq!(traced(), expected, "matching client {matching_client}");
                assert_eq!(call.result.error, 0);
                assert_eq!(
                    LOCAL.get(),
                    if matching_client {
                        ID
                    } else {
                        COMPANY_SPECTATOR
                    }
                );
            }
            reset();
        });
    }
}

/// A network client never deletes a company bankrupt for a tenth month; it updates each one's
/// admin state, keeps both companies and restores the current company.
#[cfg(test)]
mod bankruptcy_tests {
    use super::*;
    use crate::company_type::CompanyID;
    use crate::test_tables::{COMPANY, COMPANY_IDENTITY, FINANCE, SHARED};
    use std::cell::{Cell, RefCell};

    thread_local! {
        static OWNERS: RefCell<Vec<*mut Finances>> = const { RefCell::new(Vec::new()) };
        static CURRENT: Cell<u8> = const { Cell::new(18) };
        static TRACE: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }
    extern "C" fn next(from: u32) -> CompanyRef {
        OWNERS.with(|o| {
            let o = o.borrow();
            (from as usize..o.len()).find(|&i| !o[i].is_null()).map_or(
                CompanyRef {
                    finances: std::ptr::null_mut(),
                    id: INVALID,
                },
                |i| CompanyRef {
                    finances: o[i],
                    id: i as u32,
                },
            )
        })
    }
    extern "C" fn yes() -> bool {
        true
    }
    extern "C" fn no() -> bool {
        false
    }
    extern "C" fn local() -> u8 {
        0
    }
    extern "C" fn current() -> u8 {
        CURRENT.get()
    }
    extern "C" fn set_current(id: u8) {
        CURRENT.set(id);
    }
    extern "C" fn invalidate(_: u8) {}
    extern "C" fn admin_update(id: CompanyID) {
        TRACE.with(|t| t.borrow_mut().push(format!("admin {}", id.0)));
    }
    extern "C" fn month() -> Month {
        Month {
            month: 1,
            infinite_money: false,
            maintenance: false,
            fluctuating: false,
        }
    }
    fn leak<T>(value: T) -> &'static T {
        Box::leak(Box::new(value))
    }
    fn owner(money: i64, loan: i64, months: u8) -> *mut Finances {
        // SAFETY: Plain integer record; freed at the end of the test.
        let mut p: Box<Finances> = Box::new(unsafe { std::mem::zeroed() });
        p.money = money;
        p.loan = loan;
        p.bankruptcy = months;
        p.max_loan = i64::MIN; // The native default: the economy's maximum loan.
        Box::into_raw(p)
    }

    #[test]
    fn network_client_never_deletes_bankrupt_companies() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let _economy = super::financial_command_tests::ECONOMY_LOCK.lock().unwrap();
        let economy = openttd_rust_economy_state();
        // SAFETY: Process-wide economy, serialized by both locks and restored below.
        let saved = unsafe { economy.read() };
        // SAFETY: As above.
        unsafe {
            economy.write(std::mem::zeroed());
            (*economy).max_loan = 300_000;
            (*economy).fluct = 100;
        }
        OWNERS.with(|o| {
            *o.borrow_mut() = vec![
                owner(1_000_000, 0, 0),
                owner(-500_000, 100_000, 9),
                owner(-500_000, 100_000, 9),
            ];
        });
        let mut identity = COMPANY_IDENTITY;
        identity.admin_update = admin_update;
        let mut finance = FINANCE;
        finance.networking = yes;
        finance.current_company = current;
        finance.set_current_company = set_current;
        finance.invalidate_company_windows = invalidate;
        let mut s = COMPANY;
        s.identity = leak(identity);
        s.shared = &SHARED;
        s.finance = leak(finance);
        s.company_next = next;
        s.local_company = local;
        s.network_server = no;
        let s = leak(s);
        let clock = crate::timer_game_common::ClockCompany {
            services: s,
            settings: crate::test_tables::unreachable0,
            month,
            year: crate::test_tables::unreachable0,
        };
        CURRENT.set(18);
        TRACE.with(|t| t.borrow_mut().clear());
        // SAFETY: The clock context outlives the call; no exception slot is used.
        let ok = unsafe {
            crate::timer_game_common::openttd_rust_clock_economy_month_abi(
                (&raw const clock).cast(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(ok, 1);
        assert_eq!(CURRENT.get(), 18);
        assert_eq!(TRACE.with(|t| t.borrow().clone()), ["admin 1", "admin 2"]);
        OWNERS.with(|o| {
            let o = o.borrow();
            assert!(
                o.iter().all(|p| !p.is_null()),
                "a client keeps both companies"
            );
            for &p in o.iter() {
                // SAFETY: Allocated by `owner`.
                drop(unsafe { Box::from_raw(p) });
            }
        });
        // SAFETY: As above.
        unsafe { economy.write(saved) };
    }
}
