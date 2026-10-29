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
use crate::company_type::{INVALID_OWNER, MAX_COMPANIES, OWNER_DEITY, OWNER_NONE};
use crate::economy_type::EXPENSES_OTHER;
use crate::raw_field::{read_field, write_field};
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
/// `IsTileType(MP_RAILWAY)`, `IsLevelCrossingTile`, `GetTileOwner` (only for those
/// two), `HasSignals` and `GetTrackBits` (only for signalled railway).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SignalTile {
    pub railway: bool,
    pub crossing: bool,
    pub owner: u8,
    pub has_signals: bool,
    pub tracks: u8,
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
/// Caller-owned `OnTick_Companies` frame; `true` requests the `CCA_NEW_AI` Post.
#[repr(C)]
pub struct Tick {
    pub finances: *mut Finances,
    pub num_companies: usize,
    pub index: i32,
    pub timeout: i32,
    pub interval: u16,
    pub editor: bool,
    pub named: bool,
    pub competitor_due: bool,
    pub networking: bool,
    pub max_companies: u8,
    pub max_competitors: u8,
    pub num_ais: u8,
    pub stage: u8,
}
/// Inputs of the competitor timeout; the entry returns whether to Post.
#[repr(C)]
pub struct Competitors {
    pub num_companies: usize,
    pub interval: u16,
    pub menu: bool,
    pub can_start: bool,
    pub networking: bool,
    pub max_companies: u8,
    pub max_competitors: u8,
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
#[repr(C)]
pub struct Landscaping {
    pub terraform_per_64k: u32,
    pub clear_per_64k: u32,
    pub tree_per_64k: u32,
    pub object_per_64k: u32,
    pub terraform_burst: u16,
    pub clear_burst: u16,
    pub tree_burst: u16,
    pub object_burst: u16,
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
    pub finance: &'static FinanceServices,
    pub shared: &'static crate::services::Services,
    pub company_next: extern "C" fn(u32) -> CompanyRef,
    pub company_is_ai: extern "C" fn(u8) -> bool,
    pub company_count: extern "C" fn() -> usize,
    pub can_allocate_company: extern "C" fn() -> bool,
    pub local_company: extern "C" fn() -> u8,
    pub network_server: extern "C" fn() -> bool,
    pub company_vehicle_counts: extern "C" fn(u8) -> VehicleCounts,
    pub vehicle_limits: extern "C" fn() -> VehicleCounts,
    pub company_admin_update: extern "C" fn(u8),
    pub company_in_trouble: extern "C" fn(u8),
    pub post_company_delete: extern "C" fn(u8),
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
    pub airport_maintenance_cost: extern "C" fn(u8) -> i64,
    pub recession_news: extern "C" fn(bool),
    pub price_base: extern "C" fn(u32) -> PriceBase,
    pub cargo_next: extern "C" fn(u32, *mut i64) -> u32,
    pub set_cargo_payment: extern "C" fn(u32, i64),
    pub price_windows_dirty: extern "C" fn(),
    pub industry_daily_changes: extern "C" fn(bool),
    pub generate_company_name: extern "C" fn(u8),
    pub ask_merger: extern "C" fn(u8, u8, i64),
    pub is_interactive_company: extern "C" fn(u8) -> bool,
    pub show_buy_company: extern "C" fn(u8),
    pub script_random_next: extern "C" fn(u32) -> u32,
    pub reset_competitor_timeout: extern "C" fn(u32),
    pub finances_dirty: extern "C" fn(u8),
    pub show_company_finances: extern "C" fn(u8),
    pub new_year_sound: extern "C" fn(bool),
    pub generate_company_colour: extern "C" fn() -> u8,
    pub allocate_company: extern "C" fn(u8, bool) -> CompanyRef,
    pub set_company_colour: extern "C" fn(u8, u8),
    pub setup_new_company: extern "C" fn(u8, bool),
    pub new_company_events: extern "C" fn(u8),
    pub company_league_dirty: extern "C" fn(),
    pub company_ctrl_windows: extern "C" fn(),
    pub close_network_status: extern "C" fn(),
    pub network_spectate: extern "C" fn(u32),
    pub network_company_new: extern "C" fn(u8, u32, bool),
    pub network_own_company: extern "C" fn(u8, u32),
    pub assert_new_ai_slot: extern "C" fn(u8),
    pub company_bankrupt_news: extern "C" fn(u8),
    pub stop_ai: extern "C" fn(u8),
    pub delete_company: extern "C" fn(u8),
    pub company_removed: extern "C" fn(u8, u8),
    pub merger_news: extern "C" fn(u8, bool),
    pub acquisition_windows: extern "C" fn(u8),
    pub clients_to_spectators: extern "C" fn(u8),
    pub set_local_company: extern "C" fn(u8),
    pub subsidies: extern "C" fn() -> *const crate::subsidy::Services,
    pub town_next: extern "C" fn(u32, u8, *mut TownRights) -> u32,
    pub town_rating: extern "C" fn(u32, u8) -> i16,
    pub set_town_rating: extern "C" fn(u32, u8, i16, bool),
    pub set_town_exclusivity: extern "C" fn(u32, u8, u8),
    pub vehicle_next_owned: extern "C" fn(u8, u32, *mut u8) -> u32,
    pub aircraft_is_normal: extern "C" fn(u32) -> bool,
    pub vehicle_value: extern "C" fn(u32) -> i64,
    pub vehicle_is_primary: extern "C" fn(u32) -> bool,
    pub vehicle_profit: extern "C" fn(u32) -> VehicleProfit,
    pub vehicle_previous: extern "C" fn(u32) -> u32,
    pub vehicle_group_flags: extern "C" fn(u32) -> VehicleGroupFlags,
    pub vehicle_service_interval_is_custom: extern "C" fn(u32) -> bool,
    pub delete_vehicle: extern "C" fn(u32),
    pub count_group_engine: extern "C" fn(u32, i32),
    pub count_group_vehicle: extern "C" fn(u32, i32),
    pub reset_service_interval: extern "C" fn(u32, u8),
    pub set_vehicle_owner: extern "C" fn(u32, u8),
    pub assign_unit_number: extern "C" fn(u32, u8),
    pub remove_engine_replacements: extern "C" fn(u8),
    pub group_next_owned: extern "C" fn(u8, u32) -> u32,
    pub delete_group: extern "C" fn(u32),
    pub transfer_group: extern "C" fn(u32, u8),
    pub copy_service_interval_defaults: extern "C" fn(u8, u8),
    pub update_autoreplace: extern "C" fn(u8),
    pub map_size: extern "C" fn() -> u32,
    pub signal_tile: extern "C" fn(u32) -> SignalTile,
    pub has_signal_on_track: extern "C" fn(u32, u8) -> bool,
    pub add_track_to_signal_buffer: extern "C" fn(u32, u8, u8),
    pub update_signals_in_buffer: extern "C" fn(),
    pub add_airport_infrastructure: extern "C" fn(u8, u8),
    pub station_next_owned: extern "C" fn(u8, u32) -> u32,
    pub station_facility_count: extern "C" fn(u32) -> u32,
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
    pub mark_whole_screen_dirty: extern "C" fn(),
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
fn assets(s: &Services, id: u8) -> i64 {
    let mut num = 0_u32;
    each(
        |from| (s.station_next_owned)(id, from),
        |st| num = num.wrapping_add((s.station_facility_count)(st)),
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
        if kind == 0 || kind == 1 || (kind == 3 && (s.aircraft_is_normal)(v)) || kind == 2 {
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
                count = count.wrapping_add((s.station_facility_count)(st));
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
        if (s.company_is_ai)(id) {
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
            (s.company_admin_update)(id);
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
                (s.post_company_delete)(id);
                return;
            }
        }
    }
    // Original subtraction promotes uint8_t to int before ceil division.
    if (i32::from(month) + 2) / 3 != (i32::from(month) - 1 + 2) / 3 {
        (s.company_admin_update)(id);
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
    cost = cost.saturating_add((s.airport_maintenance_cost)(id));
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
    let colour = (s.generate_company_colour)();
    if f.requested != INVALID_OWNER && !(s.finance.company_finances)(f.requested).is_null() {
        return false;
    }
    let c = (s.allocate_company)(f.requested, f.is_ai);
    f.company = c.id;
    let id = c.id as u8;
    (s.set_company_colour)(id, colour);
    let loan = (((INITIAL_LOAN as u64).wrapping_mul(read_field!(e(), prices)) >> 16) / 10_000
        * 10_000) as i64;
    let loan = loan.min(read_field!(e(), max_loan));
    write_field!(c.finances, loan, loan);
    write_field!(c.finances, money, loan);
    (s.setup_new_company)(id, f.is_ai);
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
                (s.set_local_company)(c.id as u8);
                break;
            }
        }
        (f.set_current_company)(saved);
    }
    if new == INVALID_OWNER {
        write_field!((f.company_finances)(old), money, (u64::MAX >> 2) as i64);
    }
    unsafe { &*(s.subsidies)() }.transfer(old, new);
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
            (s.reset_service_interval)(v, new);
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
    let map_size = (s.map_size)();
    for tile in 0..map_size {
        // SAFETY: The outer C++ frame owns live scratch across this traversal.
        unsafe { crate::tile_owner::openttd_rust_tile_change_owner(ownership, tile, old, new) };
    }
    if new != INVALID_OWNER {
        for tile in 0..map_size {
            let t = (s.signal_tile)(tile);
            if t.railway && t.owner == new && t.has_signals {
                let mut tracks = t.tracks;
                loop {
                    // RemoveFirstTrack: INVALID_TRACK leaves NONE/INVALID bits unchanged.
                    let track = if tracks != 0 && tracks != 0xFF {
                        let first = tracks.trailing_zeros() as u8;
                        tracks &= tracks - 1;
                        first
                    } else {
                        0xFF
                    };
                    if (s.has_signal_on_track)(tile, track) {
                        (s.add_track_to_signal_buffer)(tile, track, new);
                    }
                    if tracks == 0 {
                        break;
                    }
                }
            } else if t.crossing && t.owner == new {
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
        |id| (s.delete_story_page)(id),
    );
    if new != INVALID_OWNER {
        (s.change_window_owner)(old, new);
    }
    (f.set_current_company)(current);
    (s.mark_whole_screen_dirty)();
}
fn control(s: &Services, ownership: &crate::tile_owner::Context, c: &mut Control) -> bool {
    if c.stage == 0 {
        (s.company_league_dirty)();
        match c.action {
            0 => {
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
                    (s.network_company_new)(id, c.client, true);
                    (s.network_own_company)(id, c.client);
                }
            }
            1 => {
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
            2 => {
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
                if (s.company_is_ai)(target) {
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
            (s.network_company_new)(c.startup.company as u8, 0, false);
        }
    }
    (s.company_ctrl_windows)();
    c.result = done(0, NO_EXPENSE);
    false
}
/// One iteration of the original `for (i = 0; i < max; i++)` posting loop; the
/// increment precedes the check of the resumed iteration.
fn post_competitor(t: &mut Tick) -> bool {
    if t.index >= i32::from(t.max_competitors) {
        return false;
    }
    t.index += 1;
    if t.networking {
        let old = t.num_companies;
        t.num_companies = old.wrapping_add(1);
        if old >= usize::from(t.max_companies) {
            return false;
        }
    }
    let old = t.num_ais;
    t.num_ais = old.wrapping_add(1);
    old < t.max_competitors
}
fn tick(s: &Services, t: &mut Tick) -> bool {
    if t.stage == 0 {
        if t.editor {
            return false;
        }
        let id = unsafe { TICK } as u8;
        if !t.finances.is_null() {
            if t.named {
                (s.generate_company_name)(id);
            }
            if read_field!(t.finances, asked) != 0 {
                offer(s, t.finances, id);
            }
        }
        if t.competitor_due {
            t.timeout = i32::from(t.interval)
                .wrapping_mul(60)
                .wrapping_mul(TICKS_PER_SECOND);
            if t.timeout == 0 {
                t.num_ais = count_ais(s);
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
        if post_competitor(t) {
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
    if hostile && !(s.company_is_ai)(id) {
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
        if (s.company_is_ai)(id) {
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
pub(crate) fn update_landscaping_limits(s: &Services, r: &Landscaping) {
    let next = |value: u32, per: u32, burst: u16| -> u32 {
        (u64::from(value) + u64::from(per)).min(u64::from(burst) << 16) as u32
    };
    companies(s, |p, _| {
        write_field!(
            p,
            terraform,
            next(
                read_field!(p, terraform),
                r.terraform_per_64k,
                r.terraform_burst
            )
        );
        write_field!(
            p,
            clear,
            next(read_field!(p, clear), r.clear_per_64k, r.clear_burst)
        );
        write_field!(
            p,
            tree,
            next(read_field!(p, tree), r.tree_per_64k, r.tree_burst)
        );
        write_field!(
            p,
            object,
            next(read_field!(p, object), r.object_per_64k, r.object_burst)
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
pub extern "C" fn openttd_rust_economy_add_inflation(check_year: bool, year: i32) -> bool {
    inflation(check_year, year)
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
pub extern "C" fn openttd_rust_company_startup(s: &Services, f: &mut Startup) -> bool {
    startup(s, f)
}
/// Competitor timeout callback; `true` asks the caller to Post `CCA_NEW_AI`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_competitor_timeout(s: &Services, r: &Competitors) -> bool {
    if r.menu
        || !r.can_start
        || (r.networking && r.num_companies >= usize::from(r.max_companies))
        || r.interval == 0
    {
        return false;
    }
    count_ais(s) < r.max_competitors
}
/// `OnTick_Companies`; `true` asks the caller to Post `CCA_NEW_AI` and call again.
/// # Safety
/// `t.finances` is null or the live owner of company `TICK`.
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
pub extern "C" fn openttd_rust_company_takeover_allowed(s: &Services, big: u8, small: u8) -> bool {
    takeover_limit(s, big, small)
}
/// `CmdCompanyCtrl`; `true` asks the caller to run `AI::StartNew(startup.company)`
/// and call again. The result is in `result` once it returns `false`.
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_company_control(
    s: &Services,
    ownership: &crate::tile_owner::Context,
    c: &mut Control,
) -> bool {
    control(s, ownership, c)
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
        finance,
        shared,
        company_next,
        company_is_ai,
        company_count,
        can_allocate_company,
        local_company,
        network_server,
        company_vehicle_counts,
        vehicle_limits,
        company_admin_update,
        company_in_trouble,
        post_company_delete,
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
        airport_maintenance_cost,
        recession_news,
        price_base,
        cargo_next,
        set_cargo_payment,
        price_windows_dirty,
        industry_daily_changes,
        generate_company_name,
        ask_merger,
        is_interactive_company,
        show_buy_company,
        script_random_next,
        reset_competitor_timeout,
        finances_dirty,
        show_company_finances,
        new_year_sound,
        generate_company_colour,
        allocate_company,
        set_company_colour,
        setup_new_company,
        new_company_events,
        company_league_dirty,
        company_ctrl_windows,
        close_network_status,
        network_spectate,
        network_company_new,
        network_own_company,
        assert_new_ai_slot,
        company_bankrupt_news,
        stop_ai,
        delete_company,
        company_removed,
        merger_news,
        acquisition_windows,
        clients_to_spectators,
        set_local_company,
        subsidies,
        town_next,
        town_rating,
        set_town_rating,
        set_town_exclusivity,
        vehicle_next_owned,
        aircraft_is_normal,
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
        map_size,
        signal_tile,
        has_signal_on_track,
        add_track_to_signal_buffer,
        update_signals_in_buffer,
        add_airport_infrastructure,
        station_next_owned,
        station_facility_count,
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
        mark_whole_screen_dirty,
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
    "OpenTTDCompanySignalTile" ["company_economy"] => SignalTile {
        railway,
        crossing,
        owner,
        has_signals,
        tracks,
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
    "OpenTTDCompanyTick" ["company_economy"] => Tick {
        finances,
        num_companies,
        index,
        timeout,
        interval,
        editor,
        named,
        competitor_due,
        networking,
        max_companies,
        max_competitors,
        num_ais,
        stage,
    };
    "OpenTTDCompanyCompetitors" ["company_economy"] => Competitors {
        num_companies,
        interval,
        menu,
        can_start,
        networking,
        max_companies,
        max_competitors,
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
    "OpenTTDCompanyLandscaping" ["company_economy"] => Landscaping {
        terraform_per_64k,
        clear_per_64k,
        tree_per_64k,
        object_per_64k,
        terraform_burst,
        clear_burst,
        tree_burst,
        object_burst,
    };
}
