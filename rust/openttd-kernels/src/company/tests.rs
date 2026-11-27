/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! The company and economy bodies against `company_cmd.cpp`, `economy.cpp` and `misc_cmd.cpp`.

/// The event record the native factory reads, written field by field from the original's
/// event arguments (`src/rust/services.cpp`'s factory: `company`, `other_company`, `value`),
/// so no expectation goes through the constructor it checks.
#[cfg(test)] // `tools/test-kernel-state.py` reads this file without its parent.
fn company_event(
    r#type: crate::script_event::ScriptEventType,
    company: u8,
    other_company: u8,
    value: i64,
) -> crate::script_event_types::ScriptEvent {
    crate::script_event_types::ScriptEvent {
        company,
        other_company,
        value,
        ..crate::script_event_types::ScriptEvent::of(r#type)
    }
}

/// The five financial commands against the original's recorded observations.
#[cfg(test)] // `tools/test-kernel-state.py` reads this file without its parent.
mod financial_command_tests {
    use crate::company::*;
    use crate::company_type::{CompanyID, OpenTTDCompany, Owner};
    use crate::economy_type::{ExpensesType, MAX_LOAN_LIMIT};
    use crate::services::{CommandCost, CommandCostServices};
    use crate::table::strings::{
        STR_ERROR_CURRENCY_REQUIRED, STR_ERROR_INSUFFICIENT_FUNDS, STR_ERROR_LOAN_ALREADY_REPAID,
        STR_ERROR_MAXIMUM_PERMITTED_LOAN,
    };
    use std::cell::{Cell, RefCell};

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

    /// The probe's `CommandCost`: its error enumerator (1 `CMD_ERROR`, then the four
    /// messages in `Error` order), cost, expense (255 `INVALID_EXPENSES`) and parameter.
    #[derive(Clone, Copy)]
    struct Recorded {
        error: i64,
        cost: i64,
        expense: u8,
        param: i64,
    }
    const CLEAR: Recorded = Recorded {
        error: 0,
        cost: 0,
        expense: ExpensesType::INVALID_EXPENSES.0,
        param: 0,
    };
    fn of(out: *mut CommandCost) -> &'static mut Recorded {
        // SAFETY: Every result is the test's own `Recorded`, with no other live borrow.
        unsafe { &mut *out.cast::<Recorded>() }
    }
    fn code(message: u32) -> i64 {
        match message {
            m if m == crate::strings_type::StringID::INVALID_STRING_ID.0 => 1,
            m if m == STR_ERROR_MAXIMUM_PERMITTED_LOAN.0 => 2,
            m if m == STR_ERROR_LOAN_ALREADY_REPAID.0 => 3,
            m if m == STR_ERROR_CURRENCY_REQUIRED.0 => 4,
            m if m == STR_ERROR_INSUFFICIENT_FUNDS.0 => 5,
            m => panic!("unexpected error message {m}"),
        }
    }
    extern "C" fn zero(out: *mut CommandCost) {
        *of(out) = CLEAR;
    }
    extern "C" fn error(out: *mut CommandCost, message: u32) {
        *of(out) = Recorded {
            error: code(message),
            ..CLEAR
        };
    }
    extern "C" fn error_param(out: *mut CommandCost, message: u32, param: u64) {
        *of(out) = Recorded {
            error: code(message),
            param: param as i64,
            ..CLEAR
        };
    }
    extern "C" fn expense_money(out: *mut CommandCost, expense: u8, cost: i64) {
        *of(out) = Recorded {
            cost,
            expense,
            ..CLEAR
        };
    }
    const COSTS: CommandCostServices = CommandCostServices {
        zero,
        error,
        expense_money,
        error_param,
        ..crate::test_tables::COMMAND_COSTS
    };

    thread_local! {
        static OWNERS: Cell<[*mut Finances; 3]> = const { Cell::new([std::ptr::null_mut(); 3]) };
        static CURRENT: Cell<u8> = const { Cell::new(0) };
        static NETWORKING: Cell<bool> = const { Cell::new(false) };
        static TRACE: RefCell<Vec<[i64; 3]>> = const { RefCell::new(Vec::new()) };
    }

    fn record(entry: [i64; 3]) {
        TRACE.with_borrow_mut(|t| t.push(entry));
    }
    extern "C" fn current_company() -> u8 {
        CURRENT.get()
    }
    extern "C" fn set_current_company(id: u8) {
        CURRENT.set(id);
    }
    extern "C" fn networking() -> bool {
        NETWORKING.get()
    }
    extern "C" fn company_finances(id: u8) -> *mut Finances {
        OWNERS
            .get()
            .get(usize::from(id))
            .copied()
            .unwrap_or(std::ptr::null_mut())
    }
    extern "C" fn company_valid(id: u8) -> bool {
        !company_finances(id).is_null()
    }
    extern "C" fn invalidate(id: u8) {
        record([3, i64::from(id), 0]);
    }
    extern "C" fn animation(x: i32, _: i32, _: i32, cost: i64) {
        record([53, i64::from(x / 16), cost]);
    }
    extern "C" fn colour(_: CompanyID) -> crate::gfx_type::TextColour {
        crate::gfx_type::TextColour(0)
    }
    /// The probe's `NetworkTextMessage` records the destination name's number and the amount.
    fn message(effect: &crate::news_func::testing::Effect) {
        use crate::news_func::testing::{Effect, Parameter};
        if let Effect::NetworkTextMessage { str, data, .. } = effect {
            let text = String::from_utf8(str.clone()).unwrap();
            let dest = text.rsplit(':').next().unwrap().parse::<i64>().unwrap();
            let Parameter::Integer(amount) = data else {
                panic!("give money passes an integer")
            };
            record([52, dest, *amount as i64]);
        }
    }
    extern "C" fn company(id: CompanyID) -> *mut OpenTTDCompany {
        assert!(company_valid(id.0));
        std::ptr::without_provenance_mut(usize::from(id.0) + 1)
    }
    extern "C" fn index(c: *mut OpenTTDCompany) -> CompanyID {
        CompanyID((c.addr() - 1) as u8)
    }
    extern "C" fn finances(c: *mut OpenTTDCompany) -> *mut Finances {
        company_finances(index(c).0)
    }

    const FINANCE: FinanceServices = FinanceServices {
        current_company,
        set_current_company,
        networking,
        company_finances,
        company_get_finances: company_finances,
        invalidate_company_windows: invalidate,
    };
    const IDENTITY: crate::company_identity::Services = crate::company_identity::Services {
        company,
        finances,
        index,
        ..crate::test_tables::COMPANY_IDENTITY
    };
    fn leak<T>(value: T) -> &'static T {
        Box::leak(Box::new(value))
    }
    /// The company table whose settings catalog holds `give_money` and `infinite_money`.
    fn services(give_money: &'static bool, infinite_money: &'static bool) -> Services {
        let shared = leak(crate::services::Services {
            current_company,
            company_valid,
            cost_animation: animation,
            map: crate::map_func::fixture::statics(),
            settings: leak(crate::settings_type::SettingAddresses {
                give_money,
                infinite_money,
                ..crate::settings_type::SettingAddresses::NULL
            }),
            ..crate::test_tables::SHARED
        });
        Services {
            cost: &COSTS,
            finance: &FINANCE,
            identity: &IDENTITY,
            shared,
            draw_string_company_colour: colour,
            ..crate::test_tables::COMPANY
        }
    }

    struct Run {
        old_max_loan: i64,
    }
    impl Run {
        fn start() -> Self {
            assert!(OWNERS.get().iter().all(|p| p.is_null()));
            OWNERS.set(std::array::from_fn(|_| {
                Box::into_raw(Box::new(unsafe { std::mem::zeroed::<Finances>() }))
            }));
            // A flat 64x64 map of clear tiles for `GetTilePixelZ` of the animation tile.
            let tiles = 1 << 12;
            let base: Box<[crate::map_func::BaseTileLayout]> =
                (0..tiles).map(|_| unsafe { std::mem::zeroed() }).collect();
            let extended: Box<[crate::map_func::ExtendedTileLayout]> =
                (0..tiles).map(|_| unsafe { std::mem::zeroed() }).collect();
            crate::map_func::fixture::set(
                crate::map_func::TileTypeAllocation {
                    base: Box::leak(base).as_ptr(),
                    extended: Box::leak(extended).as_ptr(),
                    count: tiles,
                    assertions: true,
                },
                6,
            );
            crate::news_func::testing::take();
            crate::news_func::testing::observe(Some(message));
            let old_max_loan = read_field!(super::super::economy(), max_loan);
            write_field!(super::super::economy(), max_loan, 300_000);
            Self { old_max_loan }
        }
    }
    impl Drop for Run {
        fn drop(&mut self) {
            crate::news_func::testing::observe(None);
            crate::news_func::testing::take();
            for owner in OWNERS.replace([std::ptr::null_mut(); 3]) {
                // SAFETY: Each owner is this test's own allocation; every access has ended.
                drop(unsafe { Box::from_raw(owner) });
            }
            TRACE.with_borrow_mut(Vec::clear);
            write_field!(super::super::economy(), max_loan, self.old_max_loan);
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
            CURRENT.set(if self.op >= 24 && variant != 0 {
                Owner::DEITY.0
            } else {
                0
            });
            NETWORKING.set(variant == 12);
            TRACE.with_borrow_mut(Vec::clear);
            for owner in OWNERS.get() {
                // SAFETY: Finances holds only zero-valid scalars and arrays; the owner is
                // exclusive and no command runs until every reset finishes.
                unsafe { owner.write(std::mem::zeroed()) };
                write_field!(owner, money, self.money);
                write_field!(owner, current_loan, self.loan);
                write_field!(
                    owner,
                    max_loan,
                    if variant == 2 {
                        MAX_LOAN_LIMIT
                    } else {
                        crate::company_base::COMPANY_MAX_LOAN_DEFAULT
                    }
                );
            }
        }

        fn invoke(&self, variant: u8) -> Recorded {
            static ON: bool = true;
            static OFF: bool = false;
            let s = services(
                if variant == 13 { &OFF } else { &ON },
                if variant == 3 { &ON } else { &OFF },
            );
            let id = if (self.op == 21 || self.op >= 24) && variant == 255 {
                255
            } else {
                1
            };
            let flags = if self.execute {
                crate::command_type::DoCommandFlag::EXECUTE
            } else {
                0
            };
            let mut result = CLEAR;
            let out = (&raw mut result).cast::<CommandCost>();
            match self.op {
                21 => openttd_rust_company_give_money(&s, out, flags, self.amount, id),
                22 => openttd_rust_company_increase_loan(&s, out, flags, variant, self.amount),
                23 => openttd_rust_company_decrease_loan(&s, out, flags, variant, self.amount),
                24 => openttd_rust_company_set_max_loan(&s, out, flags, id, self.amount),
                25 => openttd_rust_company_change_bank_balance(
                    &s,
                    out,
                    flags,
                    u32::from(variant % 2),
                    self.amount,
                    id,
                    if variant == 14 { 0 } else { variant },
                ),
                _ => unreachable!(),
            }
            result
        }
    }

    fn observation(result: Recorded) -> Vec<i64> {
        let mut values = vec![
            result.error,
            result.cost,
            i64::from(result.expense),
            result.param,
            i64::from(CURRENT.get()),
        ];
        for owner in OWNERS.get() {
            values.extend([
                read_field!(owner, money),
                read_field!(owner, current_loan),
                read_field!(owner, max_loan),
                read_field!(owner, cur_economy.income),
                read_field!(owner, cur_economy.expenses),
            ]);
            values.extend(read_field!(owner, yearly_expenses).iter().flatten());
        }
        TRACE.with_borrow(|trace| {
            values.push(trace.len() as i64);
            values.extend(trace.iter().flatten());
        });
        values
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
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
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
        let oracle = crate::oracle::parse(include_str!("financial-command-oracle.txt"));
        assert_eq!(oracle.len(), SECTIONS.len());
        for (stream, &(name, _, _)) in actual.into_iter().zip(&SECTIONS) {
            assert_eq!(crate::oracle::check(&oracle, name, stream), 40_320);
        }
    }

    /// `SubtractMoneyFromCompanyFract` resolves the payer with the checked `Company::Get`
    /// (`company_cmd.cpp:326`), never the `GetIfValid` finance lookup.
    #[test]
    fn fract_charge_uses_checked_company_get() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let _run = Run::start();
        let finance = FinanceServices {
            company_finances: crate::test_tables::FINANCE.company_finances,
            ..FINANCE
        };
        let owner = OWNERS.get()[1];
        write_field!(owner, money, 100);
        subtract_money_from_company_fract(
            &finance,
            CompanyID(1),
            0x240,
            ExpensesType::EXPENSES_ROADVEH_RUN,
        );
        assert_eq!(read_field!(owner, money_fraction), 0xC0);
        assert_eq!(read_field!(owner, money), 97);
        TRACE.with_borrow(|t| assert_eq!(*t, [[3, 1, 0]]));
    }
}

/// Road build, clear and convert price saturation, rounding and independent type masks, with
/// unclassified `NewGRF` masks and extreme prices that the default maps never load.
#[cfg(test)] // `tools/test-kernel-state.py` reads this file without its parent.
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
    use crate::company::price;
    use crate::company::testing::set_price;

    #[test]
    fn road_price_saturation_rounding_and_type_masks() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let (road_mask, tram_mask) = (
            crate::type_registry::openttd_rust_road_mask(),
            crate::type_registry::openttd_rust_tram_mask(),
        );
        // SAFETY: Process-wide scalars, serialized by the lock and restored below.
        let saved = unsafe {
            (
                price(PR_BUILD_ROAD),
                price(PR_CLEAR_ROAD),
                road_mask.replace(1 << ROADTYPE_ROAD),
                tram_mask.replace(1 << ROADTYPE_TRAM),
            )
        };
        let mut s = crate::test_tables::SHARED;
        s.road_cost_multiplier = cost_multiplier;
        set_price(PR_BUILD_ROAD, 7);
        set_price(PR_CLEAR_ROAD, -27);
        assert_eq!(build_cost(&s, ROADTYPE_ROAD), 2);
        assert_eq!(clear_cost(&s, ROADTYPE_ROAD), -27);
        assert_eq!(convert_cost(&s, ROADTYPE_ROAD, ROADTYPE_ROAD), 0);
        // Neither road nor tram: the source clear policy still earns removal value.
        // SAFETY: As above.
        unsafe { road_mask.write(0) };
        assert_eq!(clear_cost(&s, ROADTYPE_ROAD), -28);
        set_price(PR_BUILD_ROAD, -7);
        assert_eq!(build_cost(&s, ROADTYPE_ROAD), -3);
        assert_eq!(clear_cost(&s, ROADTYPE_ROAD), -25);
        set_price(PR_BUILD_ROAD, i64::MAX);
        assert_eq!(build_cost(&s, ROADTYPE_TRAM), 1_152_921_504_606_846_975);
        assert_eq!(clear_cost(&s, ROADTYPE_TRAM), -864_691_128_455_135_258);
        set_price(PR_BUILD_ROAD, i64::MIN);
        assert_eq!(build_cost(&s, ROADTYPE_TRAM), -1_152_921_504_606_846_976);
        assert_eq!(clear_cost(&s, ROADTYPE_TRAM), 864_691_128_455_135_205);
        set_price(PR_BUILD_ROAD, saved.0);
        set_price(PR_CLEAR_ROAD, saved.1);
        // SAFETY: As above.
        unsafe {
            road_mask.write(saved.2);
            tram_mask.write(saved.3);
        }
    }
}

/// Company control: the first playable company and its allocation check, service intervals,
/// ownership errors with both original assertions before error selection, and the
/// special-owner takeover mask.
#[cfg(test)] // `tools/test-kernel-state.py` reads this file without its parent.
mod control_tests {
    use crate::company::*;
    use crate::company_type::{CompanyID, CompanyMask, OWNER_END, OpenTTDCompany, Owner};
    use crate::settings_table::VehicleDefaults;
    use crate::test_tables::{COMMAND_COSTS, COMPANY, COMPANY_IDENTITY, FINANCE, SHARED};
    use crate::tile_type::TileIndex;
    use std::cell::{Cell, RefCell};

    /// Per company slot: `None` free, else whether it is an AI.
    type Slots = [Option<bool>; MAX_COMPANIES as usize];
    thread_local! {
        static SLOTS: RefCell<Slots> = const { RefCell::new([None; MAX_COMPANIES as usize]) };
        static CURRENT: Cell<u8> = const { Cell::new(0) };
        static LOCAL: Cell<u8> = const { Cell::new(0) };
        static ERRORS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
        /// Calls to `Company::CanAllocateItem`, each of which grants the native pool's
        /// allocation credit that the caller's next allocation consumes.
        static ALLOCATION_CHECKS: Cell<u32> = const { Cell::new(0) };
        /// What the next `Company::CanAllocateItem` returns, when set.
        static ALLOCATION_ANSWER: Cell<Option<bool>> = const { Cell::new(None) };
        static VEHICLE_DEFAULTS: Cell<[*mut VehicleDefaults; 2]> =
            const { Cell::new([std::ptr::null_mut(); 2]) };
    }
    /// The isolated child's native assertion results, in order.
    static FAILED: std::sync::Mutex<Vec<bool>> = std::sync::Mutex::new(Vec::new());

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
        FAILED.lock().unwrap().push(condition);
    }

    fn leak<T>(value: T) -> &'static T {
        Box::leak(Box::new(value))
    }
    fn identity() -> crate::company_identity::Services {
        crate::company_identity::Services {
            company,
            index: company_id,
            next_company,
            is_ai,
            ..COMPANY_IDENTITY
        }
    }
    fn services() -> Services {
        let shared = leak(crate::services::Services {
            company_valid,
            current_company,
            local_company,
            ..SHARED
        });
        let costs = leak(crate::services::CommandCostServices {
            zero,
            error,
            encode_owned_by,
            error_owner,
            ..COMMAND_COSTS
        });
        Services {
            identity: leak(identity()),
            shared,
            cost: costs,
            can_allocate_company: can_allocate,
            ..COMPANY
        }
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
    extern "C" fn offered_company(id: CompanyID) -> *mut OpenTTDCompany {
        if id == CompanyID(1) {
            handle(id)
        } else {
            std::ptr::null_mut()
        }
    }
    extern "C" fn offered_finances(c: *mut OpenTTDCompany) -> *mut Finances {
        assert_eq!(company_id(c), CompanyID(1));
        OFFERED.get()
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
        s.finance = leak(FinanceServices {
            networking,
            ..FINANCE
        });
        s.identity = leak(crate::company_identity::Services {
            company_if_valid: offered_company,
            finances: offered_finances,
            is_ai: human,
            ..identity()
        });
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
            offered.bankrupt_asked = CompanyMask(asked);
            for current in 16..=u8::MAX {
                CURRENT.set(current);
                for hostile in [false, true] {
                    ERRORS.with(|e| e.borrow_mut().clear());
                    cmd_buy_company(
                        &s,
                        &ownership,
                        crate::services::Cost(std::ptr::null_mut(), s.cost),
                        crate::command_type::DoCommandFlags(0),
                        CmdBuyCompany {
                            target_company: CompanyID(1),
                            hostile_takeover: hostile,
                        },
                    );
                    assert_eq!(
                        ERRORS.with(|e| e.borrow().clone()),
                        [format!(
                            "error {}",
                            crate::strings_type::StringID::INVALID_STRING_ID.0
                        )],
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
            let id = get_first_playable_company_id(&s).0;
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

    extern "C" fn client_defaults() -> *mut VehicleDefaults {
        VEHICLE_DEFAULTS.get()[1]
    }
    /// The company record the test passes; only its address is compared.
    const COMPANY_TWO: *mut OpenTTDCompany = std::ptr::dangling_mut();
    extern "C" fn company_defaults(c: *mut OpenTTDCompany) -> *mut VehicleDefaults {
        assert_eq!(c, COMPANY_TWO);
        VEHICLE_DEFAULTS.get()[0]
    }

    /// Each vehicle type reads its own interval field of the company's or, with no company,
    /// the client's defaults, ignoring the percent flag.
    #[test]
    fn service_intervals_read_the_type_field() {
        let settings = crate::settings_table::Services {
            client_vehicle_defaults: client_defaults,
            ..crate::test_tables::SETTINGS
        };
        let identity = crate::company_identity::Services {
            vehicle_defaults: company_defaults,
            ..crate::test_tables::COMPANY_IDENTITY
        };
        for percent in [false, true] {
            let mut company = VehicleDefaults {
                servint_ispercent: percent,
                servint_trains: 0,
                servint_roadveh: 121,
                servint_aircraft: 65535,
                servint_ships: 423,
            };
            let mut client = VehicleDefaults {
                servint_ispercent: !percent,
                servint_trains: 224,
                servint_roadveh: 0,
                servint_aircraft: 426,
                servint_ships: 65535,
            };
            VEHICLE_DEFAULTS.set([&raw mut company, &raw mut client]);
            for (kind, company_interval, client_interval) in [
                (crate::vehicle_type::VehicleType::VEH_TRAIN, 0, 224),
                (crate::vehicle_type::VehicleType::VEH_ROAD, 121, 0),
                (crate::vehicle_type::VehicleType::VEH_AIRCRAFT, 65535, 426),
                (crate::vehicle_type::VehicleType::VEH_SHIP, 423, 65535),
            ] {
                assert_eq!(
                    company_service_interval(&settings, &identity, Some(COMPANY_TWO), kind),
                    company_interval
                );
                assert_eq!(
                    company_service_interval(&settings, &identity, None, kind),
                    client_interval
                );
            }
            VEHICLE_DEFAULTS.set([std::ptr::null_mut(); 2]);
        }
    }

    /// The current company owns; another owner fails with the owned-by error, encoded with its
    /// owner only for the local company.
    #[test]
    fn ownership_errors() {
        let s = services();
        CURRENT.set(0);
        for observer in [0, CompanyID::SPECTATOR.0] {
            LOCAL.set(observer);
            assert_eq!(check(&s, CompanyID(0), TileIndex(1)), ["zero"]);
            for owner in [CompanyID(3), Owner::NONE] {
                let mut expected = vec![format!("error {}", STR_ERROR_OWNED_BY.0)];
                if observer == 0 {
                    expected.extend([
                        format!("encode {} 1", owner.0),
                        format!("owner {}", owner.0),
                    ]);
                }
                assert_eq!(
                    check(&s, owner, TileIndex(1)),
                    expected,
                    "observer {observer} owner {}",
                    owner.0
                );
            }
        }
    }

    /// Each original assertion fails alone before error selection; a town owner gets no error
    /// owner.
    #[test]
    fn ownership_assertions_precede_error_selection() {
        if crate::services_assert::isolate_test_check(
            "company::tests::control_tests::ownership_assertions_precede_error_selection",
            native_assert,
        ) {
            return;
        }
        let s = services();
        CURRENT.set(0);
        LOCAL.set(CompanyID::SPECTATOR.0);
        for (owner, tile) in [
            (CompanyID(OWNER_END), TileIndex(1)),
            (Owner::TOWN, TileIndex(0)),
        ] {
            FAILED.lock().unwrap().clear();
            assert_eq!(
                check(&s, owner, tile),
                [format!("error {}", STR_ERROR_OWNED_BY.0)]
            );
            assert_eq!(*FAILED.lock().unwrap(), [false], "owner {}", owner.0);
        }
        LOCAL.set(0);
        FAILED.lock().unwrap().clear();
        assert_eq!(
            check(&s, Owner::TOWN, TileIndex(0)),
            [
                format!("error {}", STR_ERROR_OWNED_BY.0),
                format!("encode {} 0", Owner::TOWN.0),
            ]
        );
        assert_eq!(*FAILED.lock().unwrap(), [false]);
    }
}

/// Competitor posts return to the caller before each post: `on_tick_companies`' return
/// protocol, driven by a loop of the native caller's form (`while on_tick(..) { Post }`).
#[cfg(test)] // `tools/test-kernel-state.py` reads this file without its parent.
mod competitor_tests {
    use crate::company::*;
    use crate::company_type::{CompanyID, OpenTTDCompany};
    use crate::openttd::GameMode;
    use crate::test_tables::{COMPANY, COMPANY_IDENTITY, FINANCE, SHARED};
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
    extern "C" fn none(_: CompanyID) -> *mut OpenTTDCompany {
        std::ptr::null_mut()
    }
    extern "C" fn end(_: u32) -> *mut OpenTTDCompany {
        std::ptr::null_mut()
    }
    extern "C" fn no_companies() -> usize {
        0
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
        let shared = leak(crate::services::Services {
            settings: leak(crate::settings_type::SettingAddresses {
                max_competitors: MAX_COMPETITORS.with(Cell::as_ptr),
                max_companies: COMPANY_LIMIT.with(Cell::as_ptr),
                competitor_interval: INTERVAL.with(Cell::as_ptr),
                ..crate::settings_type::SettingAddresses::NULL
            }),
            ..SHARED
        });
        Services {
            shared,
            finance: leak(FinanceServices {
                networking: no,
                ..FINANCE
            }),
            identity: leak(crate::company_identity::Services {
                company_if_valid: none,
                next_company: end,
                ..COMPANY_IDENTITY
            }),
            new_competitor_timeout_has_fired: yes,
            ai_can_start_new: yes,
            company_count: no_companies,
            script_random_next: no_random,
            new_competitor_timeout_reset: reset_timeout,
            ..COMPANY
        }
    }

    #[test]
    fn competitor_posts_return_before_each_post() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let s = services();
        let saved_mode = crate::session::mode();
        crate::session::set_mode(GameMode::GM_NORMAL);
        for change_limit in [false, true] {
            TRACE.with(|t| t.borrow_mut().clear());
            // SAFETY: The process-wide tick cursor, serialized by the lock.
            let cursor = unsafe { *openttd_rust_company_tick() };
            MAX_COMPETITORS.set(2);
            INTERVAL.set(0);
            let mut tick = Tick::default();
            let mut posts = 0;
            while on_tick_companies(&s, &mut tick) {
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
        crate::session::set_mode(saved_mode);
    }
}

/// Company startup and control continuations: the AI startup continuation and abandoned
/// frames, and a network client's local switch before the face and settings sync. Company 2
/// is the slot every allocation takes.
#[cfg(test)] // `tools/test-kernel-state.py` reads this file without its parent.
mod startup_tests {
    use crate::company::*;
    use crate::company_type::{CompanyCtrlAction, CompanyID, CompanyManagerFace, OpenTTDCompany};
    use crate::gfx_type::Colours;
    use crate::livery::{Livery, LiveryScheme};
    use crate::network_type::ClientID;
    use crate::news_func::testing::Effect;
    use crate::strings_type::StringID;
    use crate::test_tables::{
        COMMAND_COSTS, COMMAND_POST, COMPANY, COMPANY_IDENTITY, ENGINE, FACES, FINANCE,
        FLEET_GROUPS, MOVEMENT, NETWORK_CLIENT, NETWORK_COMMAND, NETWORK_FRAME, OBJECTIVES, SHARED,
        TYPE_AVAILABILITY,
    };
    use std::cell::{Cell, RefCell};

    const ID: u8 = 2;
    /// Company 2's native record: colour, liveries, face bits and president.
    struct Record {
        colour: Colours,
        liveries: [Livery; LiveryScheme::LS_END.0 as usize],
        face_bits: u32,
        president: (u32, u32),
    }
    thread_local! {
        static TRACE: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
        static RECORD: RefCell<Record> = RefCell::new(Record {
            colour: Colours::COLOUR_BEGIN,
            liveries: [Livery::default(); LiveryScheme::LS_END.0 as usize],
            face_bits: 0,
            president: (0, 0),
        });
        static FINANCES: Cell<*mut Finances> = const { Cell::new(std::ptr::null_mut()) };
        static NETWORKING: Cell<bool> = const { Cell::new(false) };
        static SERVER: Cell<bool> = const { Cell::new(false) };
        static CURRENT: Cell<u8> = const { Cell::new(18) };
        static LOCAL: Cell<u8> = const { Cell::new(0) };
        static OWN_CLIENT: Cell<u32> = const { Cell::new(0) };
        static POST: Cell<*const crate::command_post::Services> = const { Cell::new(std::ptr::null()) };
        static NETWORK: Cell<*const crate::network_command::Services> = const { Cell::new(std::ptr::null()) };
        static FRAME: Cell<*const crate::network_frame::Services> = const { Cell::new(std::ptr::null()) };
        static ENGINES: Cell<*const crate::engine::Services> = const { Cell::new(std::ptr::null()) };
    }
    fn trace(entry: impl Into<String>) {
        TRACE.with(|t| t.borrow_mut().push(entry.into()));
    }
    fn traced() -> Vec<String> {
        TRACE.with(|t| t.borrow().clone())
    }
    /// Trace each recorded window call and script event as it is made: `chat` for
    /// `SetLocalCompany`'s team chat invalidation, which still sees the spectator as local and
    /// current company, `classes` for a class invalidation, `windows` for the others.
    fn window_call(effect: &crate::window_func::testing::Effect) {
        use crate::window_func::testing::Effect;
        use crate::window_type::{WindowClass, WindowNumber};
        let entry = match *effect {
            Effect::InvalidateWindowData(WindowClass::WC_SEND_NETWORK_MSG, number, data, false)
                if number == WindowNumber::from(crate::network_type::DestType::DESTTYPE_TEAM)
                    && data == i32::from(CompanyID::SPECTATOR.0) =>
            {
                assert_eq!(
                    (LOCAL.get(), CURRENT.get()),
                    (CompanyID::SPECTATOR.0, CompanyID::SPECTATOR.0)
                );
                "chat"
            }
            Effect::InvalidateWindowClassesData(..) => "classes",
            _ => "window",
        };
        trace(entry);
    }
    fn news_call(effect: &Effect) {
        match effect {
            Effect::AiBroadcastNewEvent(event, skip) => {
                assert_eq!(
                    *event,
                    super::company_event(
                        crate::script_event::ScriptEventType::ET_COMPANY_NEW,
                        ID,
                        0,
                        0
                    )
                );
                trace(format!("ai event, skip {skip}"));
            }
            Effect::GameNewEvent(event) => {
                assert_eq!(
                    *event,
                    super::company_event(
                        crate::script_event::ScriptEventType::ET_COMPANY_NEW,
                        ID,
                        0,
                        0
                    )
                );
                trace("game event");
            }
            _ => {}
        }
    }
    fn handle() -> *mut OpenTTDCompany {
        std::ptr::without_provenance_mut(usize::from(ID) + 1)
    }
    extern "C" fn company(id: CompanyID) -> *mut OpenTTDCompany {
        assert_eq!(id, CompanyID(ID));
        handle()
    }
    extern "C" fn company_id(_: *mut OpenTTDCompany) -> CompanyID {
        CompanyID(ID)
    }
    extern "C" fn finances(_: *mut OpenTTDCompany) -> *mut Finances {
        FINANCES.get()
    }
    extern "C" fn no_company(_: u32) -> *mut OpenTTDCompany {
        std::ptr::null_mut()
    }
    extern "C" fn colour(_: *mut OpenTTDCompany) -> Colours {
        RECORD.with(|r| r.borrow().colour)
    }
    extern "C" fn set_colour(_: *mut OpenTTDCompany, value: Colours) {
        RECORD.with(|r| r.borrow_mut().colour = value);
    }
    extern "C" fn livery(_: *mut OpenTTDCompany) -> *mut Livery {
        RECORD.with(|r| r.borrow_mut().liveries.as_mut_ptr())
    }
    extern "C" fn face(_: *mut OpenTTDCompany) -> *mut CompanyManagerFace {
        std::ptr::without_provenance_mut(1)
    }
    extern "C" fn face_bits(_: *mut CompanyManagerFace) -> *mut u32 {
        RECORD.with(|r| &raw mut r.borrow_mut().face_bits)
    }
    extern "C" fn set_president_name_1(_: *mut OpenTTDCompany, value: StringID) {
        RECORD.with(|r| r.borrow_mut().president.0 = value.0);
    }
    extern "C" fn set_president_name_2(_: *mut OpenTTDCompany, value: u32) {
        RECORD.with(|r| r.borrow_mut().president.1 = value);
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
    extern "C" fn set_style_label(
        _: *mut CompanyManagerFace,
        _: *const crate::company_manager_face::FaceSpec,
    ) {
    }
    extern "C" fn style(_: *mut CompanyManagerFace) -> u32 {
        0
    }
    extern "C" fn no_group(_: u32) -> crate::fleet::GroupRecord {
        crate::fleet::GroupRecord {
            shell: std::ptr::null_mut(),
            group: std::ptr::null_mut(),
            statistics: std::ptr::null_mut(),
            index: 0,
        }
    }
    extern "C" fn legend() {
        trace("legend");
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
        &OBJECTIVES
    }
    extern "C" fn engines() -> *const crate::engine::Services {
        ENGINES.get()
    }
    extern "C" fn type_availability() -> *const crate::type_availability::Services {
        // SAFETY: Leaked by `services` for the test process.
        unsafe { (*ENGINES.get()).types }
    }
    extern "C" fn no_client(_: u32) -> *mut std::ffi::c_void {
        std::ptr::null_mut()
    }
    unsafe extern "C" fn no_engine(_: u32) -> crate::engine::EngineRef {
        crate::engine::EngineRef {
            shell: std::ptr::null(),
            state: std::ptr::null_mut(),
            id: 0,
            kind: crate::vehicle::VehicleType::TRAIN,
        }
    }
    extern "C" fn no_rail_type_info(
        _: crate::rail_type::RailType,
    ) -> *const crate::rail::RailTypeInfo {
        std::ptr::null()
    }
    extern "C" fn no_rail_label(_: *const crate::rail::RailTypeInfo) -> u32 {
        0
    }
    extern "C" fn no_road_info(_: crate::road_type::RoadType) -> *const crate::road::RoadTypeInfo {
        std::ptr::null()
    }
    extern "C" fn no_road_label(
        _: *const crate::road::RoadTypeInfo,
    ) -> crate::road_type::RoadTypeLabel {
        crate::road_type::RoadTypeLabel(0)
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
    extern "C" fn set_local(id: u8) {
        LOCAL.set(id);
        trace("local");
    }
    extern "C" fn yes() -> bool {
        true
    }
    extern "C" fn allocate(index: CompanyID, is_ai: bool) -> *mut OpenTTDCompany {
        assert_eq!(index, CompanyID::INVALID);
        // SAFETY: Plain integer record, freed by `reset`.
        let finances = Box::into_raw(Box::new(unsafe { std::mem::zeroed::<Finances>() }));
        FINANCES.set(finances);
        trace(format!("allocate, ai {is_ai}"));
        handle()
    }
    extern "C" fn rail_types(_: *mut OpenTTDCompany, _: crate::rail_type::RailTypes) {
        trace("rail types");
    }
    extern "C" fn road_types(_: *mut OpenTTDCompany, _: crate::road_type::RoadTypes) {
        trace("road types");
    }
    extern "C" fn inaugurated(_: *mut OpenTTDCompany, _: crate::timer_game_economy::EconomyYear) {}
    extern "C" fn inaugurated_calendar(
        _: *mut OpenTTDCompany,
        _: crate::timer_game_calendar::CalendarYear,
    ) {
    }
    extern "C" fn parse_face(_: *mut OpenTTDCompany) -> bool {
        trace("parse face");
        false
    }
    extern "C" fn default_settings(id: CompanyID) {
        assert_eq!(id, CompanyID(ID));
        trace("settings");
    }
    extern "C" fn admin_new(c: *mut OpenTTDCompany) {
        assert_eq!(c, handle());
        trace("admin");
    }
    extern "C" fn send_face(c: *mut OpenTTDCompany) {
        assert_eq!(c, handle());
        assert_eq!((LOCAL.get(), CURRENT.get()), (ID, ID));
        trace("face");
    }
    extern "C" fn sync() {
        trace("sync");
    }
    extern "C" fn close_construction() {
        trace("construction");
    }

    fn leak<T>(value: T) -> &'static T {
        Box::leak(Box::new(value))
    }
    fn shared() -> &'static crate::services::Services {
        leak(crate::services::Services {
            random: no_draw,
            utf8_length,
            screen_dirty,
            company_valid,
            current_company: current,
            local_company: local,
            set_local_company_id: set_local,
            command_post,
            network_client_info: no_client,
            type_availability,
            rail_type_info: no_rail_type_info,
            rail_type_label: no_rail_label,
            movement: leak(crate::movement::MovementServices {
                native_assert: holds,
                ..MOVEMENT
            }),
            ..SHARED
        })
    }
    fn identity(
        shared: &'static crate::services::Services,
    ) -> &'static crate::company_identity::Services {
        leak(crate::company_identity::Services {
            shared,
            faces: leak(crate::company_manager_face::Services {
                count: one_style,
                vars: no_vars,
                spec: face_spec,
                set_style,
                set_style_label,
                style,
                bits: face_bits,
                ..FACES
            }),
            groups: leak(crate::fleet::GroupServices {
                next_group: no_group,
                ..FLEET_GROUPS
            }),
            company,
            next_company: no_company,
            index: company_id,
            finances,
            colour,
            set_colour,
            livery,
            face,
            set_president_name_1,
            set_president_name_2,
            randomizer: Box::into_raw(Box::new(crate::random_func::Randomizer::default())),
            build_owner_legend: legend,
            reset_colourmap,
            ..COMPANY_IDENTITY
        })
    }
    /// The fixture's `FixtureReset`: company services over company 2's record, with the
    /// network leg (`NetworkServerNewCompany`) seeing a non-server company table.
    fn services(network: bool, is_server: bool) -> &'static Services {
        reset();
        NETWORKING.set(network);
        SERVER.set(is_server);
        let shared = shared();
        ENGINES.set(leak(crate::engine::Services {
            types: leak(crate::type_availability::Services {
                next: no_engine,
                shared,
                road_info: no_road_info,
                road_label: no_road_label,
                ..TYPE_AVAILABILITY
            }),
            ..ENGINE
        }));
        let s = leak(Services {
            identity: identity(shared),
            shared,
            finance: leak(FinanceServices {
                networking,
                current_company: current,
                set_current_company: set_current,
                ..FINANCE
            }),
            cost: &COMMAND_COSTS,
            objectives,
            engines,
            network_server: server,
            can_allocate_company: yes,
            allocate_company: allocate,
            set_avail_railtypes: rail_types,
            set_avail_roadtypes: road_types,
            set_inaugurated_year: inaugurated,
            set_inaugurated_year_calendar: inaugurated_calendar,
            parse_company_manager_face: parse_face,
            set_default_company_settings: default_settings,
            network_admin_company_new: admin_new,
            send_company_manager_face: send_face,
            sync_company_settings: sync,
            close_construction_windows: close_construction,
            ..COMPANY
        });
        // `NetworkServerNewCompany` reads the network command's own company table, which is
        // no server.
        let command = leak(crate::network_command::Services {
            company: leak(Services {
                network_server: not_server,
                ..*s
            }),
            network: network_frame,
            ..NETWORK_COMMAND
        });
        NETWORK.set(command);
        FRAME.set(leak(crate::network_frame::Services {
            command,
            client: leak(crate::network_client::Services {
                own_client_id: OWN_CLIENT.with(Cell::as_ptr),
                ..NETWORK_CLIENT
            }),
            ..NETWORK_FRAME
        }));
        POST.set(leak(crate::command_post::Services {
            network: network_command,
            ..COMMAND_POST
        }));
        s
    }
    fn reset() {
        // `GeneratePresidentName` formats through the recording `GetString`.
        crate::news_func::testing::take();
        crate::news_func::testing::observe(Some(news_call));
        crate::window_func::testing::take();
        crate::window_func::testing::observe(Some(window_call));
        TRACE.with(|t| t.borrow_mut().clear());
        CURRENT.set(18);
        LOCAL.set(0);
        let finances = FINANCES.replace(std::ptr::null_mut());
        if !finances.is_null() {
            // SAFETY: Allocated by `allocate`.
            drop(unsafe { Box::from_raw(finances) });
        }
    }
    fn finish() {
        reset();
        crate::news_func::testing::observe(None);
        crate::window_func::testing::observe(None);
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
    extern "C" fn zero(_: *mut crate::services::CommandCost) {
        trace("zero");
    }
    const COSTS: crate::services::CommandCostServices = crate::services::CommandCostServices {
        zero,
        ..COMMAND_COSTS
    };
    fn control(s: &Services, frame: &mut Control) -> bool {
        cmd_company_ctrl(
            s,
            &ownership(),
            crate::services::Cost(std::ptr::null_mut(), &COSTS),
            frame,
        )
    }
    fn control_frame(cca: CompanyCtrlAction, client: ClientID) -> Control {
        Control::new(
            crate::command_type::DoCommandFlags(crate::command_type::DoCommandFlag::EXECUTE),
            CmdCompanyCtrl {
                cca,
                company_id: CompanyID::INVALID,
                reason: crate::company_type::CompanyRemoveReason::CRR_NONE,
                client_id: client,
            },
        )
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
    /// Everything `DoStartupNewCompany` does before it asks for `AI::StartNew`.
    const STARTED: [&str; 8] = [
        "allocate, ai true",
        "rail types",
        "road types",
        "settings",
        "window",
        "window",
        "window",
        "legend",
    ];

    /// Rust returns before `AI::StartNew`; an abandoned frame keeps the allocated company
    /// without new-company events, a resumed one sends them once, and a network client starts
    /// no AI.
    #[test]
    fn ai_startup_continuation_and_abandoned_frames() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        colour_scope(|| {
            let s = services(false, false);
            let mut abandoned = Startup::new(true, CompanyID::INVALID);
            assert!(do_startup_new_company(s, &mut abandoned));
            assert_eq!(abandoned.index, u32::from(ID));
            let mut expected: Vec<String> = STARTED.iter().map(|&e| e.to_owned()).collect();
            expected.push("window".to_owned());
            assert_eq!(traced(), expected);
            assert_eq!(
                read_field!(FINANCES.get(), money),
                read_field!(FINANCES.get(), current_loan)
            );

            let s = services(false, false);
            let mut resumed = Startup::new(true, CompanyID::INVALID);
            let mut starts = 0;
            while do_startup_new_company(s, &mut resumed) {
                assert_eq!(resumed.index, u32::from(ID));
                trace("start");
                starts += 1;
            }
            assert_eq!(starts, 1);
            expected.extend(["start", "ai event, skip 2", "game event"].map(str::to_owned));
            assert_eq!(traced(), expected);

            // A command's AI startup returns to the caller the same way, then announces the
            // company and invalidates the script windows.
            let s = services(false, false);
            let mut call =
                control_frame(CompanyCtrlAction::CCA_NEW_AI, ClientID::INVALID_CLIENT_ID);
            while control(s, &mut call) {
                trace("start");
            }
            let mut control_expected = vec!["window".to_owned()];
            control_expected.extend(STARTED.iter().map(|&e| e.to_owned()));
            control_expected.extend(
                [
                    "window",
                    "start",
                    "ai event, skip 2",
                    "game event",
                    "admin",
                    "classes",
                    "classes",
                    "classes",
                    "zero",
                ]
                .map(str::to_owned),
            );
            assert_eq!(traced(), control_expected);

            // A network client starts no AI: the events follow at once.
            let s = services(true, false);
            let mut client = Startup::new(true, CompanyID::INVALID);
            assert!(!do_startup_new_company(s, &mut client));
            let mut client_expected: Vec<String> = STARTED.iter().map(|&e| e.to_owned()).collect();
            client_expected.extend(["window", "ai event, skip 2", "game event"].map(str::to_owned));
            assert_eq!(traced(), client_expected);
            finish();
        });
    }

    /// A network client's new company switches the local company before the face and
    /// settings sync, and only when the client asked for it.
    #[test]
    fn network_local_switch_precedes_face_and_settings_sync() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        colour_scope(|| {
            for matching_client in [false, true] {
                let s = services(true, false);
                LOCAL.set(CompanyID::SPECTATOR.0);
                CURRENT.set(CompanyID::SPECTATOR.0);
                OWN_CLIENT.set(if matching_client { 7 } else { 8 });
                let mut call = control_frame(CompanyCtrlAction::CCA_NEW, ClientID(7));
                assert!(!control(s, &mut call));
                let mut expected = vec!["window".to_owned(), "window".to_owned()];
                expected.extend(STARTED.iter().map(|&e| e.replace("ai true", "ai false")));
                expected.extend(
                    ["window", "ai event, skip 2", "game event", "admin"].map(str::to_owned),
                );
                if matching_client {
                    expected.extend(
                        [
                            "chat",
                            "local",
                            "classes",
                            "construction",
                            "screen",
                            "classes",
                            "classes",
                            "classes",
                            "colourmap",
                            "face",
                            "sync",
                            "screen",
                        ]
                        .map(str::to_owned),
                    );
                }
                expected.extend(["classes", "classes", "classes", "zero"].map(str::to_owned));
                assert_eq!(traced(), expected, "matching client {matching_client}");
                assert_eq!(
                    LOCAL.get(),
                    if matching_client {
                        ID
                    } else {
                        CompanyID::SPECTATOR.0
                    }
                );
            }
            finish();
        });
    }
}

/// A network client never deletes a company bankrupt for a tenth month; it updates each one's
/// admin state and keeps both companies.
#[cfg(test)] // `tools/test-kernel-state.py` reads this file without its parent.
mod bankruptcy_tests {
    use crate::company::*;
    use crate::company_type::{CompanyID, OpenTTDCompany};
    use crate::test_tables::{COMPANY, COMPANY_IDENTITY, FINANCE, SHARED};
    use std::cell::RefCell;

    thread_local! {
        static OWNERS: RefCell<Vec<*mut Finances>> = const { RefCell::new(Vec::new()) };
        static TRACE: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }
    fn handle(id: usize) -> *mut OpenTTDCompany {
        std::ptr::without_provenance_mut(id + 1)
    }
    extern "C" fn next(from: u32) -> *mut OpenTTDCompany {
        OWNERS.with(|o| {
            let o = o.borrow();
            (from as usize..o.len())
                .find(|&i| !o[i].is_null())
                .map_or(std::ptr::null_mut(), handle)
        })
    }
    extern "C" fn index(c: *mut OpenTTDCompany) -> CompanyID {
        CompanyID((c.addr() - 1) as u8)
    }
    extern "C" fn finances(c: *mut OpenTTDCompany) -> *mut Finances {
        OWNERS.with(|o| o.borrow()[usize::from(index(c).0)])
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
    extern "C" fn admin_update(c: *mut OpenTTDCompany) {
        TRACE.with(|t| t.borrow_mut().push(format!("admin {}", index(c).0)));
    }
    fn leak<T>(value: T) -> &'static T {
        Box::leak(Box::new(value))
    }
    fn owner(money: i64, loan: i64, months: u8) -> *mut Finances {
        // SAFETY: Plain integer record; freed at the end of the test.
        let mut p: Box<Finances> = Box::new(unsafe { std::mem::zeroed() });
        p.money = money;
        p.current_loan = loan;
        p.months_of_bankruptcy = months;
        // The native default: the economy's maximum loan.
        p.max_loan = crate::company_base::COMPANY_MAX_LOAN_DEFAULT;
        Box::into_raw(p)
    }

    #[test]
    fn network_client_never_deletes_bankrupt_companies() {
        static FINITE: bool = false;
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let economy = openttd_rust_economy_state();
        let saved = read_field!(economy, max_loan);
        write_field!(economy, max_loan, 300_000);
        OWNERS.with(|o| {
            *o.borrow_mut() = vec![
                owner(1_000_000, 0, 0),
                owner(-500_000, 100_000, 9),
                owner(-500_000, 100_000, 9),
            ];
        });
        let s = Services {
            identity: leak(crate::company_identity::Services {
                admin_update,
                next_company: next,
                index,
                finances,
                ..COMPANY_IDENTITY
            }),
            shared: leak(crate::services::Services {
                local_company: local,
                settings: leak(crate::settings_type::SettingAddresses {
                    infinite_money: &raw const FINITE,
                    ..crate::settings_type::SettingAddresses::NULL
                }),
                ..SHARED
            }),
            finance: leak(FinanceServices {
                networking: yes,
                ..FINANCE
            }),
            network_server: no,
            ..COMPANY
        };
        TRACE.with(|t| t.borrow_mut().clear());
        companies(&s, |c| company_check_bankrupt(&s, c));
        assert_eq!(TRACE.with(|t| t.borrow().clone()), ["admin 1", "admin 2"]);
        OWNERS.with(|o| {
            let o = o.borrow();
            for (&p, months) in o.iter().zip([0, 10, 10]) {
                assert_eq!(read_field!(p, months_of_bankruptcy), months);
                // SAFETY: Allocated by `owner`.
                drop(unsafe { Box::from_raw(p) });
            }
        });
        write_field!(economy, max_loan, saved);
    }
}

/// The plan's new tests (#1048, Evidence): news, events, windows and native assertions, which
/// the harness's game-state comparison cannot see. Companies 0 to 2 are handles carrying their
/// index over owned finance records; each observed call joins one trace in order.
#[cfg(test)] // `tools/test-kernel-state.py` reads this file without its parent.
mod evidence_tests {
    use crate::company::*;
    use crate::company_type::{CompanyID, CompanyMask, OpenTTDCompany, Owner};
    use crate::economy_type::{ExpensesType, Price, PriceCategory};
    use crate::news_func::testing::{Effect, Param, company_news_name, encoded};
    use crate::test_tables::{
        COMMANDS, COMPANY, COMPANY_IDENTITY, FINANCE, FLEET_GROUPS, MOVEMENT, OBJECTIVES, SETTINGS,
        SHARED, STATION_COMMANDS, STATION_WORLD, SUBSIDY, VEHICLE,
    };
    use std::cell::{Cell, RefCell};

    const COUNT: usize = 3;
    thread_local! {
        static FINANCES: Cell<[*mut Finances; COUNT]> = const { Cell::new([std::ptr::null_mut(); COUNT]) };
        static AI: Cell<[bool; COUNT]> = const { Cell::new([false; COUNT]) };
        static TRACE: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
        static CURRENT: Cell<u8> = const { Cell::new(0) };
        static LOCAL: Cell<u8> = const { Cell::new(0) };
        static TOWNS: RefCell<Vec<*mut crate::town::State>> = const { RefCell::new(Vec::new()) };
        /// The one group's Rust owner, which also stands for its shell.
        static GROUP: Cell<*mut ()> = const { Cell::new(std::ptr::null_mut()) };
        static STATION_OWNER: Cell<u8> = const { Cell::new(0) };
        static RENEWALS: Cell<*mut ()> = const { Cell::new(std::ptr::null_mut()) };
        static STATS: Cell<*mut ()> = const { Cell::new(std::ptr::null_mut()) };
        static INFRASTRUCTURE: Cell<[*mut crate::company_base::CompanyInfrastructure; COUNT]> =
            const { Cell::new([std::ptr::null_mut(); COUNT]) };
        static DEFAULTS: Cell<[*mut crate::settings_table::VehicleDefaults; COUNT]> =
            const { Cell::new([std::ptr::null_mut(); COUNT]) };
        /// The vehicle pool: each record, null once destroyed.
        static VEHICLES: RefCell<Vec<*mut crate::vehicle_base::VehicleRecord>> =
            const { RefCell::new(Vec::new()) };
    }
    fn trace(entry: impl Into<String>) {
        TRACE.with_borrow_mut(|t| t.push(entry.into()));
    }
    fn traced() -> Vec<String> {
        TRACE.with_borrow_mut(std::mem::take)
    }
    fn handle(id: CompanyID) -> *mut OpenTTDCompany {
        std::ptr::without_provenance_mut(usize::from(id.0) + 1)
    }
    fn finances_of(id: usize) -> *mut Finances {
        FINANCES.get()[id]
    }
    fn valid(id: u8) -> bool {
        usize::from(id) < COUNT && !finances_of(usize::from(id)).is_null()
    }
    extern "C" fn company(id: CompanyID) -> *mut OpenTTDCompany {
        assert!(valid(id.0), "Company::Get({})", id.0);
        handle(id)
    }
    extern "C" fn company_if_valid(id: CompanyID) -> *mut OpenTTDCompany {
        if valid(id.0) {
            handle(id)
        } else {
            std::ptr::null_mut()
        }
    }
    extern "C" fn index(c: *mut OpenTTDCompany) -> CompanyID {
        CompanyID((c.addr() - 1) as u8)
    }
    extern "C" fn finances(c: *mut OpenTTDCompany) -> *mut Finances {
        finances_of(usize::from(index(c).0))
    }
    extern "C" fn next_company(from: u32) -> *mut OpenTTDCompany {
        (from as usize..COUNT)
            .find(|&id| !finances_of(id).is_null())
            .map_or(std::ptr::null_mut(), |id| handle(CompanyID(id as u8)))
    }
    extern "C" fn is_ai(c: *mut OpenTTDCompany) -> bool {
        AI.get()[usize::from(index(c).0)]
    }
    extern "C" fn company_valid(id: u8) -> bool {
        valid(id)
    }
    extern "C" fn company_finances(id: u8) -> *mut Finances {
        if valid(id) {
            finances_of(usize::from(id))
        } else {
            std::ptr::null_mut()
        }
    }
    extern "C" fn current_company() -> u8 {
        CURRENT.get()
    }
    extern "C" fn set_current_company(id: u8) {
        CURRENT.set(id);
    }
    extern "C" fn local_company() -> u8 {
        LOCAL.get()
    }
    extern "C" fn no() -> bool {
        false
    }
    extern "C" fn invalidate(_: u8) {}
    extern "C" fn admin_update(c: *mut OpenTTDCompany) {
        trace(format!("admin {}", index(c).0));
    }
    extern "C" fn renewal_head(_: *const OpenTTDCompany) -> *mut crate::fleet::EngineRenewList {
        RENEWALS.with(Cell::as_ptr).cast()
    }
    extern "C" fn town_next_id(from: u32) -> u32 {
        let count = TOWNS.with_borrow(Vec::len) as u32;
        if from < count { from } else { u32::MAX }
    }
    extern "C" fn town_owner_by_id(id: u32) -> *mut crate::town::State {
        TOWNS.with_borrow(|t| t[id as usize])
    }
    extern "C" fn vehicle_next(from: u32) -> Option<crate::vehicle::VehicleRef> {
        VEHICLES.with_borrow(|v| {
            v.iter()
                .skip(from as usize)
                .find(|p| !p.is_null())
                .map(|&p| crate::vehicle::VehicleRef(std::ptr::NonNull::new(p.cast()).unwrap()))
        })
    }
    unsafe extern "C" fn vehicle_id(v: *mut crate::vehicle::OpenTTDVehicle) -> u32 {
        VEHICLES.with_borrow(|all| all.iter().position(|&p| p.cast() == v).unwrap() as u32)
    }
    extern "C" fn destroy_vehicle(v: crate::vehicle::VehicleRef) {
        // SAFETY: A live record of the fixture's pool.
        let id = unsafe { vehicle_id(v.0.as_ptr()) };
        trace(format!("destroy vehicle {id}"));
        // `delete v` of a head frees its chain: each part leaves the pool.
        let mut part = v.0.as_ptr().cast::<crate::vehicle_base::VehicleRecord>();
        while !part.is_null() {
            let next = read_field!(part, next);
            VEHICLES.with_borrow_mut(|all| {
                for slot in all.iter_mut().filter(|p| **p == part) {
                    *slot = std::ptr::null_mut();
                }
            });
            // SAFETY: The test's own record, out of the pool and no longer referenced.
            unsafe { crate::vehicle_base::VehicleRecord::delete(part) };
            part = next;
        }
    }
    extern "C" fn screen_dirty() {
        trace("screen");
    }
    extern "C" fn infrastructure(id: u8) -> *mut crate::company_base::CompanyInfrastructure {
        INFRASTRUCTURE.get()[usize::from(id)]
    }
    /// `Station::Iterate()` over the one station, whose service record is never read here.
    extern "C" fn for_each_station(context: *mut c_void, visit: crate::services::StationVisitor) {
        // SAFETY: The visitor and its context come from `Services::for_each_station`.
        unsafe {
            visit(
                context,
                std::ptr::without_provenance(1),
                std::ptr::null_mut(),
                0,
            );
        }
    }
    extern "C" fn station_owner(_: *mut crate::station::BaseStation) -> u8 {
        STATION_OWNER.get()
    }
    extern "C" fn set_station_owner(_: *mut crate::station::BaseStation, owner: u8) {
        STATION_OWNER.set(owner);
        trace(format!("station owner {owner}"));
    }
    extern "C" fn station_commands() -> *const crate::station_cmd::CommandServicesTable {
        const WORLD: crate::station_service::World = crate::station_service::World {
            station_owner,
            ..STATION_WORLD
        };
        // The command table whose company getter the group bodies reach the company table by.
        const COMMAND: crate::services::CommandServices = crate::services::CommandServices {
            company: fleet_company,
            ..COMMANDS
        };
        const TABLE: crate::station_cmd::CommandServicesTable =
            crate::station_cmd::CommandServicesTable {
                station: std::ptr::from_ref(&WORLD),
                set_station_owner,
                command: std::ptr::from_ref(&COMMAND),
                ..STATION_COMMANDS
            };
        std::ptr::from_ref(&TABLE)
    }
    extern "C" fn newgrf() -> *const crate::newgrf_vehicle::Services {
        const TABLE: crate::newgrf_vehicle::Services =
            crate::newgrf_vehicle::TEST_TABLE.with_id(vehicle_id);
        std::ptr::from_ref(&TABLE)
    }
    extern "C" fn next_group(from: u32) -> crate::fleet::GroupRecord {
        let group = GROUP.get();
        if from == 0 && !group.is_null() {
            crate::fleet::GroupRecord {
                shell: group.cast(),
                group: group.cast(),
                statistics: STATS.get().cast(),
                index: 0,
            }
        } else {
            crate::fleet::GroupRecord {
                shell: std::ptr::null_mut(),
                group: std::ptr::null_mut(),
                statistics: std::ptr::null_mut(),
                index: 0,
            }
        }
    }
    extern "C" fn delete_group(shell: *mut crate::fleet::OpenTTDGroup) {
        assert_eq!(shell.cast(), GROUP.get());
        trace("delete group 0");
        GROUP.set(std::ptr::null_mut());
    }
    extern "C" fn stats(
        _: CompanyID,
        _: crate::group_type::GroupID,
        _: crate::vehicle_type::VehicleType,
    ) -> *mut crate::fleet::StatsFields {
        STATS.get().cast()
    }
    extern "C" fn fleet_company() -> *const Services {
        thread_local! {
            static TABLE: &'static Services = leak(services());
        }
        TABLE.with(|t| std::ptr::from_ref(*t))
    }
    extern "C" fn group_all(
        _: *mut OpenTTDCompany,
        _: crate::vehicle_type::VehicleType,
    ) -> *mut () {
        STATS.get()
    }
    extern "C" fn use_free_group(c: *mut OpenTTDCompany) -> u16 {
        trace(format!("free group of {}", index(c).0));
        7
    }
    extern "C" fn subsidies() -> *const crate::subsidy::Services {
        thread_local! {
            static TABLE: &'static crate::subsidy::Services = leak(crate::subsidy::Services {
                shared: shared(),
                next_industry,
                clear_source_flags,
                ..SUBSIDY
            });
        }
        TABLE.with(|t| *t)
    }
    extern "C" fn next_industry(_: u32) -> u32 {
        u32::MAX
    }
    extern "C" fn clear_source_flags(_: crate::subsidy::Source) {}
    extern "C" fn settings() -> &'static crate::settings_table::Services {
        const TABLE: crate::settings_table::Services = crate::settings_table::Services {
            company_vehicle_defaults,
            ..SETTINGS
        };
        &TABLE
    }
    extern "C" fn company_vehicle_defaults(
        c: CompanyID,
    ) -> *mut crate::settings_table::VehicleDefaults {
        DEFAULTS.get()[usize::from(c.0)]
    }
    /// `&c->settings.vehicle` of a resolved company.
    extern "C" fn vehicle_defaults(
        c: *mut OpenTTDCompany,
    ) -> *mut crate::settings_table::VehicleDefaults {
        DEFAULTS.get()[usize::from(index(c).0)]
    }
    extern "C" fn end(_: u32) -> crate::company::PoolCursor {
        crate::company::PoolCursor {
            item: std::ptr::null_mut(),
            index: u32::MAX,
        }
    }
    extern "C" fn objectives() -> &'static crate::objectives::Services {
        &OBJECTIVES
    }
    extern "C" fn change_window_owner(old: Owner, new: Owner) {
        trace(format!("window owner {} {}", old.0, new.0));
    }
    extern "C" fn ai_stop(c: CompanyID) {
        trace(format!("ai stop {}", c.0));
    }
    extern "C" fn close_company_windows(c: CompanyID) {
        trace(format!("close windows {}", c.0));
    }
    extern "C" fn delete_company(c: *mut OpenTTDCompany) {
        trace(format!("delete {}", index(c).0));
    }
    extern "C" fn show_buy_company_dialog(c: CompanyID, hostile: bool) {
        trace(format!("buy dialog {} {hostile}", c.0));
    }
    /// The news and event effects in the trace, by their recorded form.
    fn news_call(effect: &Effect) {
        if !matches!(effect, Effect::GetString(..)) {
            trace(format!("{effect:?}"));
        }
    }
    fn window_call(effect: &crate::window_func::testing::Effect) {
        trace(format!("{effect:?}"));
    }

    fn leak<T>(value: T) -> &'static T {
        Box::leak(Box::new(value))
    }
    fn shared() -> &'static crate::services::Services {
        static MAX: u16 = 500;
        static FINITE: bool = false;
        thread_local! {
            static TABLE: &'static crate::services::Services = leak(crate::services::Services {
                company_valid,
                current_company,
                local_company,
                town_next_id,
                town_owner_by_id,
                vehicle_next,
                screen_dirty,
                company_infrastructure: infrastructure,
                for_each_station,
                station_commands,
                map: crate::map_func::fixture::statics(),
                settings: leak(crate::settings_type::SettingAddresses {
                    infinite_money: &raw const FINITE,
                    max_trains: &raw const MAX,
                    max_roadveh: &raw const MAX,
                    max_ships: &raw const MAX,
                    max_aircraft: &raw const MAX,
                    ..crate::settings_type::SettingAddresses::NULL
                }),
                movement: leak(crate::movement::MovementServices {
                    vehicle: leak(crate::vehicle::VehicleServices {
                        newgrf,
                        destroy: destroy_vehicle,
                        ..VEHICLE
                    }),
                    train: &crate::test_tables::TRAIN_LEAVES,
                    ..MOVEMENT
                }),
                ..SHARED
            });
        }
        TABLE.with(|t| *t)
    }
    fn services() -> Services {
        let shared = shared();
        Services {
            shared,
            finance: leak(FinanceServices {
                current_company,
                set_current_company,
                networking: no,
                company_finances,
                invalidate_company_windows: invalidate,
                ..FINANCE
            }),
            identity: leak(crate::company_identity::Services {
                shared,
                commands: leak(crate::services::CommandServices { shared, ..COMMANDS }),
                groups: leak(crate::fleet::GroupServices {
                    shared,
                    next_group,
                    group_statistics_get: stats,
                    group_delete: delete_group,
                    ..FLEET_GROUPS
                }),
                company,
                company_if_valid,
                index,
                finances,
                next_company,
                is_ai,
                admin_update,
                renewal_head,
                vehicle_defaults,
                ..COMPANY_IDENTITY
            }),
            subsidies,
            settings,
            objectives,
            group_all,
            group_default: group_all,
            use_free_group,
            next_waypoint: end,
            next_sign: end,
            next_goal: end,
            next_story_page: end,
            change_window_owner,
            ai_stop,
            close_company_windows,
            delete_company,
            show_buy_company_dialog,
            network_server: no,
            ..COMPANY
        }
    }

    /// The world's records, owned for one test; dropped in place.
    struct World {
        finances: Vec<Finances>,
        towns: Vec<crate::town::State>,
        group: *mut (),
        stats: *mut (),
        infrastructure: Vec<crate::company_base::CompanyInfrastructure>,
        defaults: Vec<crate::settings_table::VehicleDefaults>,
    }
    impl World {
        fn new() -> Self {
            // SAFETY: Plain integer records.
            let mut world = Self {
                finances: (0..COUNT).map(|_| unsafe { std::mem::zeroed() }).collect(),
                towns: Vec::new(),
                group: {
                    let g = crate::fleet::openttd_rust_fleet_group_create();
                    let fields = g.cast::<crate::fleet::GroupFields>();
                    write_field!(fields, owner, 1);
                    write_field!(fields, vehicle_type, 0);
                    write_field!(fields, number, 3);
                    g
                },
                stats: crate::fleet::openttd_rust_fleet_stats_create(),
                infrastructure: (0..COUNT).map(|_| unsafe { std::mem::zeroed() }).collect(),
                defaults: (0..COUNT)
                    .map(|id| crate::settings_table::VehicleDefaults {
                        servint_ispercent: id == 0,
                        servint_trains: 100 + id as u16,
                        servint_roadveh: 200 + id as u16,
                        servint_aircraft: 300 + id as u16,
                        servint_ships: 400 + id as u16,
                    })
                    .collect(),
            };
            FINANCES.set(std::array::from_fn(|i| &raw mut world.finances[i]));
            INFRASTRUCTURE.set(std::array::from_fn(|i| &raw mut world.infrastructure[i]));
            DEFAULTS.set(std::array::from_fn(|i| &raw mut world.defaults[i]));
            GROUP.set(world.group);
            STATS.set(world.stats);
            AI.set([false; COUNT]);
            RENEWALS.set(std::ptr::null_mut());
            STATION_OWNER.set(1);
            CURRENT.set(0);
            LOCAL.set(0);
            for (owner, value) in world.infrastructure.iter_mut().zip([4, 5, 6]) {
                owner.airport = value;
            }
            crate::map_func::fixture::set_dimensions(0, 0);
            crate::news_func::testing::take();
            crate::news_func::testing::observe(Some(news_call));
            crate::window_func::testing::take();
            crate::window_func::testing::observe(Some(window_call));
            TRACE.with_borrow_mut(Vec::clear);
            world
        }
        fn town(&mut self, have: u16, ratings: [i16; 3], exclusivity: u8, counter: u8) {
            let mut t = crate::town::State::default();
            t.simulation.have_ratings = have;
            t.simulation.ratings[..3].copy_from_slice(&ratings);
            t.simulation.exclusivity = exclusivity;
            t.simulation.exclusive_counter = counter;
            self.towns.push(t);
            TOWNS.set(self.towns.iter_mut().map(|t| &raw mut *t).collect());
        }
    }
    impl Drop for World {
        fn drop(&mut self) {
            crate::news_func::testing::observe(None);
            crate::news_func::testing::take();
            crate::window_func::testing::observe(None);
            crate::window_func::testing::take();
            FINANCES.set([std::ptr::null_mut(); COUNT]);
            INFRASTRUCTURE.set([std::ptr::null_mut(); COUNT]);
            DEFAULTS.set([std::ptr::null_mut(); COUNT]);
            GROUP.set(std::ptr::null_mut());
            STATS.set(std::ptr::null_mut());
            TOWNS.set(Vec::new());
            for v in VEHICLES.take() {
                if !v.is_null() {
                    // SAFETY: Created by the test; no access is live.
                    unsafe { crate::vehicle_base::VehicleRecord::delete(v) };
                }
            }
            // SAFETY: Created by `World::new`; no access is live.
            unsafe {
                crate::fleet::openttd_rust_fleet_stats_destroy(self.stats);
                crate::fleet::openttd_rust_fleet_group_destroy(self.group);
            }
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
    fn get(s: &Services, id: u8) -> CompanyRef {
        CompanyRef::get(s.identity, CompanyID(id))
    }
    fn effect(e: &Effect) -> String {
        format!("{e:?}")
    }
    fn window(e: crate::window_func::testing::Effect) -> String {
        format!("{e:?}")
    }

    /// Evidence 1: At the fourth month of a bad balance, `CompanyCheckBankrupt` warns: the company's
    /// news with its headline, then the AI and Game events, and only then the admin update.
    #[test]
    fn bankrupt_month_four_warns_before_the_admin_update() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let _world = World::new();
        let s = services();
        let f = finances_of(1);
        write_field!(f, money, -500_000);
        write_field!(f, current_loan, 100_000);
        write_field!(f, months_of_bankruptcy, 3);
        write_field!(f, max_loan, crate::company_base::COMPANY_MAX_LOAN_DEFAULT);
        let saved = read_field!(economy(), max_loan);
        write_field!(economy(), max_loan, 300_000);
        company_check_bankrupt(&s, get(&s, 1));
        write_field!(economy(), max_loan, saved);
        let c = handle(CompanyID(1));
        let name = company_news_name(c);
        assert_eq!(
            traced(),
            [
                effect(&Effect::CompanyNewsInformation(
                    crate::table::strings::STR_NEWS_COMPANY_IN_TROUBLE_TITLE.0,
                    c,
                    std::ptr::null()
                )),
                effect(&Effect::News {
                    headline: encoded(
                        crate::table::strings::STR_NEWS_COMPANY_IN_TROUBLE_DESCRIPTION.0,
                        &[Param::S(&name)]
                    ),
                    r#type: crate::news_type::NewsType::CompanyInfo,
                    style: crate::news_type::NewsStyle::Company,
                    flags: crate::news_type::NewsFlags::default(),
                    ref1: crate::news_type::NewsReference::None,
                    ref2: crate::news_type::NewsReference::None,
                    data: Some(name.clone()),
                    advice_type: crate::news_type::AdviceType::Invalid,
                }),
                effect(&Effect::AiBroadcastNewEvent(
                    super::company_event(
                        crate::script_event::ScriptEventType::ET_COMPANY_IN_TROUBLE,
                        1,
                        0,
                        0
                    ),
                    CompanyID::INVALID.0
                )),
                effect(&Effect::GameNewEvent(super::company_event(
                    crate::script_event::ScriptEventType::ET_COMPANY_IN_TROUBLE,
                    1,
                    0,
                    0
                ))),
                "admin 1".to_owned(),
            ]
        );
        assert_eq!(read_field!(f, months_of_bankruptcy), 4);
    }

    /// The walk's calls of a takeover of company 1 by company 0 after its towns, in order.
    fn takeover_walk() -> Vec<String> {
        [
            "free group of 0",
            "station owner 0",
            "window owner 1 0",
            "screen",
        ]
        .map(str::to_owned)
        .to_vec()
    }

    /// Evidence 3: `DoAcquireCompany`, hostile and not: the merger news with both names and, when not
    /// hostile, the value; the two merger events of the old company and the buyer; the
    /// transfer; `AI::Stop` only for an AI; the windows; and the deletion last.
    #[test]
    fn acquire_company_news_events_transfer_and_deletion_in_order() {
        use crate::window_func::testing::Effect as W;
        use crate::window_type::{WindowClass, WindowNumber};
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        for (hostile, ai) in [(false, false), (false, true), (true, true)] {
            let mut world = World::new();
            world.town(0, [0; 3], 0, 0);
            let s = services();
            AI.set([false, ai, false]);
            write_field!(finances_of(1), bankrupt_value, 12_345);
            do_acquire_company(&s, &ownership(), get(&s, 1), hostile);
            let (c, other) = (handle(CompanyID(1)), handle(CompanyID(0)));
            let (name, other_name) = (company_news_name(c), company_news_name(other));
            let mut params = vec![Param::S(&name), Param::S(&other_name)];
            let string = if hostile {
                crate::table::strings::STR_NEWS_MERGER_TAKEOVER_TITLE
            } else {
                params.push(Param::N(12_345));
                crate::table::strings::STR_NEWS_COMPANY_MERGER_DESCRIPTION
            };
            let mut expected = vec![
                effect(&Effect::CompanyNewsInformation(
                    crate::table::strings::STR_NEWS_COMPANY_MERGER_TITLE.0,
                    c,
                    other,
                )),
                effect(&Effect::News {
                    headline: encoded(string.0, &params),
                    r#type: crate::news_type::NewsType::CompanyInfo,
                    style: crate::news_type::NewsStyle::Company,
                    flags: crate::news_type::NewsFlags::default(),
                    ref1: crate::news_type::NewsReference::None,
                    ref2: crate::news_type::NewsReference::None,
                    data: Some(name.clone()),
                    advice_type: crate::news_type::AdviceType::Invalid,
                }),
                effect(&Effect::AiBroadcastNewEvent(
                    super::company_event(
                        crate::script_event::ScriptEventType::ET_COMPANY_MERGER,
                        1,
                        0,
                        0,
                    ),
                    CompanyID::INVALID.0,
                )),
                effect(&Effect::GameNewEvent(super::company_event(
                    crate::script_event::ScriptEventType::ET_COMPANY_MERGER,
                    1,
                    0,
                    0,
                ))),
            ];
            expected.extend(takeover_walk());
            if ai {
                expected.push("ai stop 1".to_owned());
            }
            expected.push("close windows 1".to_owned());
            for class in [
                WindowClass::WC_TRAINS_LIST,
                WindowClass::WC_SHIPS_LIST,
                WindowClass::WC_ROADVEH_LIST,
                WindowClass::WC_AIRCRAFT_LIST,
            ] {
                expected.push(window(W::InvalidateWindowClassesData(class, 0, false)));
            }
            expected.push(window(W::InvalidateWindowData(
                WindowClass::WC_CLIENT_LIST,
                WindowNumber(0),
                0,
                false,
            )));
            expected.push("delete 1".to_owned());
            assert_eq!(traced(), expected, "hostile {hostile}");
            assert_eq!(CURRENT.get(), 0);
            drop(world);
        }
    }

    /// Evidence 4: `HandleBankruptcyTakeover` asks the best-performing company that has not been
    /// asked, with the value, and shows the dialog only to the local company.
    #[test]
    fn bankruptcy_takeover_asks_the_best_company() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        for local in [0, 2] {
            let world = World::new();
            let s = services();
            LOCAL.set(local);
            write_field!(finances_of(0), old_economy[1].performance_history, 10);
            write_field!(finances_of(2), old_economy[1].performance_history, 20);
            let f = finances_of(1);
            let mut asked = CompanyMask::default();
            asked.set(CompanyID(1));
            write_field!(f, bankrupt_asked, asked);
            write_field!(f, bankrupt_value, 777);
            handle_bankruptcy_takeover(&s, get(&s, 1));
            let mut expected = vec![effect(&Effect::AiNewEvent(
                2,
                super::company_event(
                    crate::script_event::ScriptEventType::ET_COMPANY_ASK_MERGER,
                    1,
                    0,
                    777,
                ),
            ))];
            if local == 2 {
                expected.push("buy dialog 1 false".to_owned());
            }
            assert_eq!(traced(), expected, "local {local}");
            assert!(read_field!(f, bankrupt_asked).test(CompanyID(2)));
            assert_eq!(
                i32::from(read_field!(f, bankrupt_timeout)),
                3 * 30 * crate::timer_game_tick::DAY_TICKS / 14
            );
            drop(world);
        }
    }

    /// Evidence 4: With no bit of `bankrupt_asked` set, `HandleBankruptcyTakeover` ends at the native
    /// assertion.
    #[test]
    fn bankruptcy_takeover_without_an_offer_fails_the_assertion() {
        if crate::services_assert::isolate_test_exit(
            "company::tests::evidence_tests::bankruptcy_takeover_without_an_offer_fails_the_assertion",
        ) {
            return;
        }
        let world = World::new();
        let s = services();
        handle_bankruptcy_takeover(&s, get(&s, 1));
        drop(world);
        unreachable!("the assertion ends the child");
    }

    /// Evidence 5: `ChangeOwnershipOfCompanyItems` for a takeover and for a deletion over two towns, one
    /// group, one station and a map of no tiles.
    #[test]
    fn change_ownership_moves_ratings_rights_groups_and_stations() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let initial = RATING_INITIAL as i16;
        // A takeover: the new owner gets the old ratings, or the better of both, and the old
        // owner's exclusivity.
        let mut world = World::new();
        world.town(0b010, [0, 500, 0], 1, 3);
        world.town(0b011, [100, 700, 0], 2, 5);
        let s = services();
        change_ownership_of_company_items(&s, &ownership(), CompanyID(1), CompanyID(0));
        let towns: Vec<_> = world
            .towns
            .iter()
            .map(|t| {
                (
                    t.simulation.have_ratings,
                    t.simulation.ratings[0],
                    t.simulation.ratings[1],
                    t.simulation.exclusivity,
                    t.simulation.exclusive_counter,
                )
            })
            .collect();
        assert_eq!(
            towns,
            [(0b001, 500, initial, 0, 3), (0b001, 700, initial, 2, 5)]
        );
        let group = world.group.cast::<crate::fleet::GroupFields>();
        assert_eq!(
            (read_field!(group, owner), read_field!(group, number)),
            (0, 7)
        );
        assert_eq!(STATION_OWNER.get(), 0);
        assert_eq!(world.infrastructure[0].airport, 4 + 5);
        assert_eq!(world.defaults[1].servint_trains, 100);
        assert!(world.defaults[1].servint_ispercent);
        assert_eq!(traced(), takeover_walk());
        drop(world);

        // A deletion: no ratings move, the rights end, the group goes and the station is
        // nobody's; the company holds the jackpot while its property is removed. Of its
        // two-part train, only the part with no previous link, the head, is deleted.
        let mut world = World::new();
        world.town(0b010, [0, 500, 0], 1, 3);
        world.town(0b011, [100, 700, 0], 2, 5);
        let parts = [0, 1].map(|_| {
            let v = crate::vehicle_base::VehicleRecord::new(
                std::ptr::null_mut(),
                crate::vehicle_type::VehicleType::VEH_TRAIN,
            );
            write_field!(v, owner, CompanyID(1));
            v
        });
        // SAFETY: Both records are the test's own.
        unsafe { crate::vehicle_base::set_next(parts[0], parts[1]) };
        VEHICLES.set(parts.to_vec());
        let s = services();
        change_ownership_of_company_items(&s, &ownership(), CompanyID(1), Owner::INVALID);
        let towns: Vec<_> = world
            .towns
            .iter()
            .map(|t| {
                (
                    t.simulation.have_ratings,
                    t.simulation.ratings[0],
                    t.simulation.ratings[1],
                    t.simulation.exclusivity,
                    t.simulation.exclusive_counter,
                )
            })
            .collect();
        assert_eq!(
            towns,
            [
                (0b000, 0, initial, CompanyID::INVALID.0, 0),
                (0b001, 100, initial, 2, 5)
            ]
        );
        assert_eq!(
            traced(),
            [
                "destroy vehicle 0",
                "delete group 0",
                "station owner 16",
                "screen"
            ]
            .map(str::to_owned)
        );
        assert_eq!(STATION_OWNER.get(), Owner::NONE.0);
        assert_eq!(world.finances[1].money, (u64::MAX >> 2) as i64);
        assert_eq!(world.infrastructure[0].airport, 4);
        drop(world);
    }

    /// Evidence 5: `ChangeOwnershipOfCompanyItems` with the same old and new owner ends at the native
    /// assertion.
    #[test]
    fn change_ownership_to_the_same_owner_fails_the_assertion() {
        if crate::services_assert::isolate_test_exit(
            "company::tests::evidence_tests::change_ownership_to_the_same_owner_fails_the_assertion",
        ) {
            return;
        }
        let world = World::new();
        let s = services();
        LOCAL.set(2);
        change_ownership_of_company_items(&s, &ownership(), CompanyID(1), CompanyID(1));
        drop(world);
        unreachable!("the assertion ends the child");
    }

    extern "C" fn start_price(p: Price) -> i64 {
        match p.0 {
            0 => 1,
            1 => -1,
            _ => 1_000,
        }
    }
    extern "C" fn category(_: Price) -> PriceCategory {
        PriceCategory::PCAT_RUNNING
    }
    fn price_services(costs: &'static u8) -> Services {
        static LOAN: u32 = 300_000;
        static CONSTRUCTION: u8 = 1;
        Services {
            shared: leak(crate::services::Services {
                settings: leak(crate::settings_type::SettingAddresses {
                    max_loan: &raw const LOAN,
                    vehicle_costs: costs,
                    construction_cost: &raw const CONSTRUCTION,
                    ..crate::settings_type::SettingAddresses::NULL
                }),
                ..SHARED
            }),
            price_base_start_price: start_price,
            price_base_category: category,
            ..COMPANY
        }
    }

    /// Evidence 6: `RecomputePrices` never stores a zero price: a start price of 1 shifted to zero
    /// stores 1, and one of −1 stores −1.
    #[test]
    fn recompute_prices_keeps_each_price_off_zero() {
        static COSTS: u8 = 1;
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let s = price_services(&COSTS);
        let e = economy();
        let saved = (
            read_field!(e, max_loan),
            read_field!(e, inflation_prices),
            read_field!(e, inflation_payment),
        );
        let prices: Vec<i64> = (0..Price::PR_END.0).map(|i| price(Price(i))).collect();
        write_field!(e, inflation_prices, 1 << 16);
        write_field!(e, inflation_payment, 1 << 16);
        reset_price_base_multipliers();
        set_price_base_multiplier(Price(0), -2);
        set_price_base_multiplier(Price(1), -2);
        crate::window_func::testing::take();
        crate::cargo_registry::install_test_no_cargo();
        recompute_prices(&s);
        crate::cargo_registry::clear_test_labels();
        crate::window_func::testing::take();
        assert_eq!(
            (price(Price(0)), price(Price(1)), price(Price(2))),
            (1, -1, 1_000)
        );
        assert_eq!(read_field!(e, max_loan), 300_000);
        reset_price_base_multipliers();
        for (i, value) in prices.into_iter().enumerate() {
            set_price(Price(i as u8), value);
        }
        write_field!(e, max_loan, saved.0);
        write_field!(e, inflation_prices, saved.1);
        write_field!(e, inflation_payment, saved.2);
    }

    const ENDED: i32 = 87;
    const ENDED_MARKER: &str = "openttd-rust test: NOT_REACHED";
    /// Evidence 6: A difficulty of 3 ends at the native unreachable path.
    #[test]
    fn recompute_prices_of_an_unknown_difficulty_is_not_reached() {
        static COSTS: u8 = 3;
        extern "C" fn not_reached() {
            eprintln!("{ENDED_MARKER}");
            std::process::exit(ENDED);
        }
        extern "C" fn holds(condition: bool) {
            assert!(condition);
        }
        if crate::services_assert::isolate_test_ends(
            "company::tests::evidence_tests::recompute_prices_of_an_unknown_difficulty_is_not_reached",
            ENDED,
            ENDED_MARKER,
        ) {
            return;
        }
        let mut s = price_services(&COSTS);
        s.shared = leak(crate::services::Services {
            not_reached,
            map: crate::map_func::fixture::statics(),
            ..*s.shared
        });
        // SAFETY: The leaked shared table and this thread's map statics live through the child.
        unsafe { crate::services_assert::bind_test_policy(holds, s.shared) };
        recompute_prices(&s);
        unreachable!("NOT_REACHED ends the child");
    }

    /// Evidence 7: `SubtractMoneyFromAnyCompany` returns at a zero cost before its assertion, and ends
    /// at the assertion for `INVALID_EXPENSES` with a cost.
    #[test]
    fn subtracting_an_invalid_expense_fails_the_assertion() {
        if crate::services_assert::isolate_test_exit(
            "company::tests::evidence_tests::subtracting_an_invalid_expense_fails_the_assertion",
        ) {
            return;
        }
        let world = World::new();
        let s = services();
        let f = finances_of(1);
        // SAFETY: The world's live record of company 1.
        unsafe {
            subtract_money_from_any_company(
                s.finance,
                f,
                CompanyID(1),
                0,
                ExpensesType::INVALID_EXPENSES,
            );
            subtract_money_from_any_company(
                s.finance,
                f,
                CompanyID(1),
                5,
                ExpensesType::INVALID_EXPENSES,
            );
        }
        drop(world);
        unreachable!("the assertion ends the child");
    }

    /// The command result of the offline deletion's `Command::Post`, held in its native
    /// storage: its error message (0 for success), cost and expense type.
    #[derive(Clone, Copy)]
    struct Posted {
        error: u32,
        cost: i64,
        expense: ExpensesType,
    }
    fn posted(out: *const crate::services::CommandCost) -> Posted {
        // SAFETY: Every result is a `Posted` that `construct` wrote into its storage.
        unsafe { out.cast::<Posted>().read() }
    }
    fn store(out: *mut crate::services::CommandCost, value: Posted) {
        // SAFETY: As `posted`.
        unsafe { out.cast::<Posted>().write(value) };
    }
    const SUCCESS: Posted = Posted {
        error: 0,
        cost: 0,
        expense: ExpensesType::INVALID_EXPENSES,
    };
    unsafe extern "C" fn construct(
        storage: *mut crate::services::CommandCostStorage,
    ) -> *mut crate::services::CommandCost {
        let out = storage.cast::<crate::services::CommandCost>();
        store(out, SUCCESS);
        out
    }
    unsafe extern "C" fn destroy(_: *mut crate::services::CommandCost) {}
    extern "C" fn zero(out: *mut crate::services::CommandCost) {
        store(out, SUCCESS);
    }
    extern "C" fn error(out: *mut crate::services::CommandCost, message: u32) {
        store(
            out,
            Posted {
                error: message,
                ..SUCCESS
            },
        );
    }
    extern "C" fn move_cost(
        to: *mut crate::services::CommandCost,
        from: *mut crate::services::CommandCost,
    ) {
        store(to, posted(from));
    }
    extern "C" fn success(out: *const crate::services::CommandCost) -> bool {
        posted(out).error == 0
    }
    extern "C" fn money(out: *const crate::services::CommandCost) -> i64 {
        posted(out).cost
    }
    extern "C" fn expense(out: *const crate::services::CommandCost) -> ExpensesType {
        posted(out).expense
    }
    const POST_COSTS: crate::services::CommandCostServices = crate::services::CommandCostServices {
        construct,
        destroy,
        zero,
        error,
        move_cost,
        success,
        money,
        expense,
        ..crate::test_tables::COMMAND_COSTS
    };
    extern "C" fn holds(condition: bool) {
        assert!(condition);
    }
    extern "C" fn local_is_18() -> u8 {
        18
    }
    extern "C" fn count() -> usize {
        (0..COUNT).filter(|&id| !finances_of(id).is_null()).count()
    }
    extern "C" fn delete_and_free(c: *mut OpenTTDCompany) {
        let id = usize::from(index(c).0);
        trace(format!("delete {id}"));
        let mut slots = FINANCES.get();
        slots[id] = std::ptr::null_mut();
        FINANCES.set(slots);
    }
    extern "C" fn admin_remove(c: CompanyID, reason: crate::company_type::CompanyRemoveReason) {
        trace(format!("admin remove {} {}", c.0, reason.0));
    }
    extern "C" fn no_pages() -> u32 {
        0
    }
    extern "C" fn counted_objectives() -> &'static crate::objectives::Services {
        const TABLE: crate::objectives::Services = crate::objectives::Services {
            page_count: no_pages,
            goal_count: no_pages,
            ..OBJECTIVES
        };
        &TABLE
    }
    extern "C" fn no_desync_logging() -> bool {
        false
    }
    extern "C" fn control_begin(
        _: *mut crate::command_post::ControlStorage,
    ) -> *const crate::tile_owner::Context {
        leak(ownership())
    }
    extern "C" fn control_end(_: *mut crate::command_post::ControlStorage) {}
    thread_local! {
        static POST: Cell<*const crate::command_post::Services> = const { Cell::new(std::ptr::null()) };
        static COMMAND: Cell<*const crate::services::CommandServices> = const { Cell::new(std::ptr::null()) };
        static POST_COMPANY: Cell<*const Services> = const { Cell::new(std::ptr::null()) };
    }
    extern "C" fn command_post() -> &'static crate::command_post::Services {
        // SAFETY: Leaked by the test for the process.
        unsafe { &*POST.get() }
    }
    extern "C" fn command() -> &'static crate::services::CommandServices {
        // SAFETY: As above.
        unsafe { &*COMMAND.get() }
    }
    extern "C" fn post_company() -> *const Services {
        POST_COMPANY.get()
    }

    /// The services of the offline economy month: the company table over the world, whose
    /// shared table posts through a command table that runs `CmdCompanyCtrl` over a copy of
    /// it. As the native fixture's post table did, that copy's local company is 18, the
    /// current company the command's completion switches to.
    fn month_services() -> &'static Services {
        static FINITE: bool = false;
        let base = services();
        let shared = leak(crate::services::Services {
            command_post,
            settings: leak(crate::settings_type::SettingAddresses {
                infinite_money: &raw const FINITE,
                infrastructure_maintenance: &raw const FINITE,
                economy: &raw const FINITE,
                ..crate::settings_type::SettingAddresses::NULL
            }),
            ..*base.shared
        });
        let s = leak(Services {
            shared,
            objectives: counted_objectives,
            company_count: count,
            delete_company: delete_and_free,
            company_admin_remove: admin_remove,
            ..base
        });
        POST_COMPANY.set(leak(Services {
            shared: leak(crate::services::Services {
                local_company: local_is_18,
                ..*shared
            }),
            ..*s
        }));
        COMMAND.set(leak(crate::services::CommandServices {
            finance: s.finance,
            company: post_company,
            shared,
            cost: &POST_COSTS,
            signals: leak(crate::signal::SignalServices {
                assert_condition: holds,
                ..crate::test_tables::SIGNALS
            }),
            shift_pressed: no,
            ..COMMANDS
        }));
        POST.set(leak(crate::command_post::Services {
            command,
            control_begin,
            control_end,
            desync_logging: no_desync_logging,
            ..crate::test_tables::COMMAND_POST
        }));
        s
    }
    fn bankrupt(id: usize) {
        let f = finances_of(id);
        write_field!(f, money, -500_000);
        write_field!(f, current_loan, 100_000);
        write_field!(f, months_of_bankruptcy, 9);
        write_field!(f, max_loan, crate::company_base::COMPANY_MAX_LOAN_DEFAULT);
    }
    /// Run the economy month callback once, with the economy and clock it reads.
    fn economy_month(s: &'static Services) {
        let e = economy();
        // SAFETY: All-zero is the record's value-initialised state; restored below.
        let saved = std::mem::replace(unsafe { &mut *e }, unsafe { std::mem::zeroed() });
        write_field!(e, max_loan, 300_000);
        write_field!(e, fluct, 100);
        let month = crate::timer_game_common::economy_month();
        crate::timer_game_common::write_economy_month(1);
        let clock = crate::timer_game_common::ClockCompany { services: s };
        // SAFETY: The clock context outlives the call; no exception slot is used.
        let ok = unsafe {
            crate::timer_game_common::openttd_rust_clock_economy_month_abi(
                (&raw const clock).cast(),
                std::ptr::null_mut(),
            )
        };
        assert_eq!(ok, 1);
        crate::timer_game_common::write_economy_month(month);
        // SAFETY: As above.
        unsafe { e.write(saved) };
    }

    /// Evidence 2: offline, one economy month deletes both companies bankrupt for a tenth
    /// month inside the one callback, in ID order, each through `Command::Post` and
    /// `CmdCompanyCtrl` with `CRR_BANKRUPT` and no admin update after it, and the current
    /// company is 18 again afterwards. A local company is kept, asked of everyone, with its
    /// admin update instead.
    #[test]
    fn offline_bankrupt_deletion_executes_in_id_order() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let world = World::new();
        let s = month_services();
        GROUP.set(std::ptr::null_mut());
        write_field!(finances_of(0), money, 1_000_000);
        bankrupt(1);
        bankrupt(2);
        CURRENT.set(18);
        economy_month(s);
        let bankrupt_reason = crate::company_type::CompanyRemoveReason::CRR_BANKRUPT.0;
        let calls: Vec<String> = traced()
            .into_iter()
            .filter(|t| t.starts_with("delete") || t.starts_with("admin"))
            .collect();
        assert_eq!(
            calls,
            [
                "delete 1".to_owned(),
                format!("admin remove 1 {bankrupt_reason}"),
                "delete 2".to_owned(),
                format!("admin remove 2 {bankrupt_reason}"),
            ]
        );
        assert_eq!(CURRENT.get(), 18);
        assert_eq!(count(), 1);
        drop(world);

        // The local company is not deleted offline: it is asked of everyone.
        let world = World::new();
        let s = month_services();
        GROUP.set(std::ptr::null_mut());
        bankrupt(1);
        LOCAL.set(1);
        CURRENT.set(18);
        economy_month(s);
        let calls: Vec<String> = traced()
            .into_iter()
            .filter(|t| t.starts_with("delete") || t.starts_with("admin"))
            .collect();
        assert_eq!(calls, ["admin 1"]);
        assert!(read_field!(finances_of(1), bankrupt_asked).all());
        assert_eq!(count(), 3);
        drop(world);
    }

    /// A takeover vehicle the test owns: its record, consist and `NewGRF` cache.
    type Part = (
        *mut crate::vehicle_base::VehicleRecord,
        Box<crate::orders::Consist>,
        Box<crate::newgrf_vehicle::Cache>,
    );
    thread_local! {
        /// A takeover's vehicles, whose native shells carry their pool index.
        static SHELLS: RefCell<Vec<Part>> = const { RefCell::new(Vec::new()) };
    }
    fn shell_index(shell: *mut ()) -> usize {
        shell.addr() - 1
    }
    thread_local! {
        /// The takeover's vehicle pool descriptor, whose address the scheduler table names.
        static TAKEOVER_POOL_VIEW: Cell<*const crate::pool::PoolDescriptor> =
            const { Cell::new(std::ptr::null()) };
        static TAKEOVER_ORDERS: Cell<*const crate::orders::Leaves> =
            const { Cell::new(std::ptr::null()) };
    }
    extern "C" fn takeover_orders() -> *const crate::orders::Leaves {
        TAKEOVER_ORDERS.get()
    }
    extern "C" fn never(_: crate::vehicle::VehicleRef) -> bool {
        false
    }
    extern "C" fn no_profit(_: crate::vehicle::VehicleRef) -> i64 {
        0
    }
    extern "C" fn young(_: crate::vehicle::VehicleRef) -> i32 {
        0
    }
    extern "C" fn default_group(_: crate::vehicle::VehicleRef) -> crate::group_type::GroupID {
        crate::group_type::GroupID::DEFAULT_GROUP
    }
    extern "C" fn record_from_shell(
        shell: *mut std::ffi::c_void,
    ) -> *mut crate::vehicle::OpenTTDVehicle {
        trace(format!("resolve {}", shell_index(shell.cast())));
        SHELLS.with_borrow(|s| s[shell_index(shell.cast())].0.cast())
    }
    extern "C" fn set_colourmap(_: crate::vehicle::VehicleRef, _: crate::gfx_type::PaletteID) {}
    unsafe extern "C" fn cache(
        v: *mut crate::vehicle::OpenTTDVehicle,
    ) -> *mut crate::newgrf_vehicle::Cache {
        SHELLS.with_borrow_mut(|s| {
            let entry = s.iter_mut().find(|e| e.0.cast() == v).unwrap();
            &raw mut *entry.2
        })
    }
    extern "C" fn set_interval(shell: *mut (), interval: u16, custom: bool, percent: bool) {
        trace(format!(
            "interval {} {interval} {custom} {percent} current {}",
            shell_index(shell),
            CURRENT.get()
        ));
    }
    extern "C" fn use_free_unit(
        _: *mut OpenTTDCompany,
        _: crate::vehicle_type::VehicleType,
    ) -> u16 {
        1
    }
    extern "C" fn set_unitnumber(_: crate::vehicle::VehicleRef, _: u16) {}
    thread_local! {
        static TAKEOVER: Cell<*const Services> = const { Cell::new(std::ptr::null()) };
        static TAKEOVER_SETTINGS: Cell<*const crate::settings_table::Services> =
            const { Cell::new(std::ptr::null()) };
    }
    extern "C" fn takeover_company() -> *const Services {
        TAKEOVER.get()
    }
    extern "C" fn takeover_settings() -> &'static crate::settings_table::Services {
        // SAFETY: Leaked by `takeover_services` for the test process.
        unsafe { &*TAKEOVER_SETTINGS.get() }
    }
    /// The company table of a takeover whose vehicles run the nested `CmdChangeServiceInt`
    /// through the real `command::nested` and `vehicle_transactions::cmd_change_service_int`.
    fn takeover_services() -> &'static Services {
        let base = services();
        let vehicles = leak(crate::vehicle::VehicleServices {
            newgrf: takeover_newgrf,
            destroy: destroy_vehicle,
            record_from_shell,
            set_colourmap,
            orders: takeover_orders,
            profit_last_year: no_profit,
            maintenance: leak(crate::vehicle::maintenance::Services {
                economy_age: young,
                ..crate::test_tables::MAINTENANCE
            }),
            ..VEHICLE
        });
        let shared = leak(crate::services::Services {
            movement: leak(crate::movement::MovementServices {
                vehicle: vehicles,
                train: &crate::test_tables::TRAIN_LEAVES,
                ..MOVEMENT
            }),
            ..*base.shared
        });
        let groups = leak(crate::fleet::GroupServices {
            vehicles,
            is_engine_countable: never,
            group_id: default_group,
            ..*base.identity.groups
        });
        // The vehicle pool: each of `SHELLS` in the slot of its index.
        let slots: Vec<_> = SHELLS.with_borrow(|shells| {
            shells
                .iter()
                .enumerate()
                .map(|(id, part)| crate::pool::PoolSlot {
                    shell: std::ptr::without_provenance_mut(id + 1),
                    record: part.0.cast(),
                })
                .collect()
        });
        let first_unused = slots.len();
        let pool = Box::into_raw(Box::new(crate::pool::PoolDescriptor {
            slots: crate::industrytype::NativeVec::from_vec(slots),
            used_bitmap: crate::industrytype::NativeVec::new(),
            first_free: 0,
            first_unused,
            items: 0,
            checked: 0,
            cleaning: false,
            alloc_cache: std::ptr::null_mut(),
        }));
        TAKEOVER_POOL_VIEW.set(pool);
        TAKEOVER_ORDERS.set(leak(crate::orders::Leaves {
            base: vehicles,
            scheduler: leak(crate::vehicle::scheduler::SchedulerServices {
                pool: TAKEOVER_POOL_VIEW.with(|view| leak(view.get())),
                ..crate::test_tables::SCHEDULER
            }),
            ..crate::test_tables::ORDERS
        }));
        let commands = leak(crate::services::CommandServices {
            shared,
            finance: base.finance,
            company: takeover_company,
            cost: &POST_COSTS,
            signals: leak(crate::signal::SignalServices {
                assert_condition: holds,
                ..crate::test_tables::SIGNALS
            }),
            ..COMMANDS
        });
        let transactions = leak(crate::vehicle_transactions::TransactionServices {
            shared,
            commands,
            cost: &POST_COSTS,
            groups,
            set_interval,
            ..crate::test_tables::VEHICLE_TRANSACTION
        });
        TAKEOVER_SETTINGS.set(leak(crate::settings_table::Services {
            transactions,
            company_vehicle_defaults,
            ..SETTINGS
        }));
        let s = leak(Services {
            shared,
            cost: &POST_COSTS,
            identity: leak(crate::company_identity::Services {
                groups,
                shared,
                ..*base.identity
            }),
            settings: takeover_settings,
            use_free_unit,
            set_unitnumber,
            ..base
        });
        TAKEOVER.set(s);
        s
    }
    extern "C" fn takeover_newgrf() -> *const crate::newgrf_vehicle::Services {
        const TABLE: crate::newgrf_vehicle::Services = crate::newgrf_vehicle::TEST_TABLE
            .with_id(vehicle_id)
            .with_cache(cache);
        std::ptr::from_ref(&TABLE)
    }
    /// A road vehicle front engine of `owner`, with a custom service interval or not.
    fn road_vehicle(owner: CompanyID, custom: bool) {
        SHELLS.with_borrow_mut(|s| {
            let id = s.len();
            let v = crate::vehicle_base::VehicleRecord::new(
                std::ptr::without_provenance_mut(id + 1),
                crate::vehicle_type::VehicleType::VEH_ROAD,
            );
            // SAFETY: Plain integer records.
            let mut consist: Box<crate::orders::Consist> = Box::new(unsafe { std::mem::zeroed() });
            if custom {
                consist.vehicle_flags =
                    1 << crate::base_consist::VehicleFlag::SERVICE_INTERVAL_IS_CUSTOM.0;
            }
            write_field!(v, consist, &raw mut *consist);
            write_field!(v, owner, owner);
            write_field!(
                v,
                subtype,
                1 << crate::vehicle_base::GroundVehicleSubtypeFlags::GVSF_FRONT.0
            );
            s.push((v, consist, Box::default()));
            VEHICLES.with_borrow_mut(|all| all.push(v));
        });
    }

    /// `ChangeOwnershipOfCompanyItems` on a takeover (`economy.cpp:447-453`): each vehicle of
    /// the old owner whose interval is not custom runs the nested `CMD_CHANGE_SERVICE_INT`
    /// with the old owner as the current company, executed, not custom, and gets the buyer's
    /// default; a vehicle with a custom interval runs none.
    #[test]
    fn takeover_resets_each_non_custom_service_interval() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        let world = World::new();
        GROUP.set(std::ptr::null_mut());
        road_vehicle(CompanyID(1), false);
        road_vehicle(CompanyID(1), true);
        road_vehicle(CompanyID(2), false);
        road_vehicle(CompanyID(1), false);
        let s = takeover_services();
        change_ownership_of_company_items(s, &ownership(), CompanyID(1), CompanyID(0));
        let intervals: Vec<String> = traced()
            .into_iter()
            .filter(|t| t.starts_with("interval") || t.starts_with("resolve"))
            .collect();
        // Company 0's road vehicle default is 200, its percent flag set; the walk first
        // copies both to company 1's defaults, which the command reads. Outside a command
        // the nested call is top-level: its test pass resolves the vehicle's record from its
        // shell (twice, for the primary and ownership checks), then its execution resolves it
        // again and stores. The custom vehicle 1 runs no command and is never resolved.
        assert_eq!(
            intervals,
            [
                "resolve 0",
                "resolve 0",
                "resolve 0",
                "resolve 0",
                "interval 0 200 false true current 1",
                "resolve 3",
                "resolve 3",
                "resolve 3",
                "resolve 3",
                "interval 3 200 false true current 1",
            ]
        );
        SHELLS.with_borrow(|shells| {
            for (i, owner) in [0, 0, 2, 0].into_iter().enumerate() {
                assert_eq!(
                    read_field!(shells[i].0, owner),
                    CompanyID(owner),
                    "vehicle {i}"
                );
            }
        });
        SHELLS.with_borrow_mut(Vec::clear);
        drop(world);
    }
}
