/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! The identity bodies' calls in order, over recording company records and the recording
//! window, news and event services. Companies are handles carrying their index; every
//! observed call pushes one line to the thread's trace, so an order across services is one
//! trace. Expectations follow `company_cmd.cpp`.
use super::*;
use crate::news_func::testing::Effect as News;
use crate::news_func::testing::{Param, encoded};
use crate::test_tables::{COMMANDS, COMPANY_IDENTITY, FINANCE, FLEET_GROUPS, SHARED};
use crate::window_func::testing::Effect as Window;
use std::cell::{Cell, RefCell};

/// One company's native members.
struct Record {
    colour: Colours,
    is_ai: bool,
    last_build_coordinate: TileIndex,
    livery: [Livery; LiveryScheme::LS_END.0 as usize],
    name: Vec<u8>,
    name_1: StringID,
    name_2: u32,
    president_name: Vec<u8>,
    president_name_1: StringID,
    president_name_2: u32,
}
impl Record {
    fn new(colour: Colours) -> Self {
        Self {
            colour,
            is_ai: false,
            last_build_coordinate: TileIndex(0),
            livery: [Livery::default(); LiveryScheme::LS_END.0 as usize],
            name: Vec::new(),
            name_1: STR_SV_UNNAMED,
            name_2: 0,
            president_name: Vec::new(),
            president_name_1: StringID(0),
            president_name_2: 0,
        }
    }
}

thread_local! {
    static COMPANIES: RefCell<Vec<Record>> = const { RefCell::new(Vec::new()) };
    static TRACE: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static CURRENT: Cell<u8> = const { Cell::new(0) };
    static DRAWS: Cell<u32> = const { Cell::new(0) };
    static VEHICLES: RefCell<Vec<VehicleRef>> = const { RefCell::new(Vec::new()) };
}

fn trace(line: impl Into<String>) {
    TRACE.with(|t| t.borrow_mut().push(line.into()));
}
fn handle(id: usize) -> *mut OpenTTDCompany {
    std::ptr::without_provenance_mut(id + 1)
}
fn id(c: *const OpenTTDCompany) -> usize {
    c.addr() - 1
}
fn with<R>(c: *const OpenTTDCompany, f: impl FnOnce(&mut Record) -> R) -> R {
    COMPANIES.with(|r| f(&mut r.borrow_mut()[id(c)]))
}
fn text(bytes: &[u8]) -> std::string::String {
    std::string::String::from_utf8_lossy(bytes).into_owned()
}
fn span(bytes: &[u8]) -> ByteSpan {
    ByteSpan {
        data: bytes.as_ptr(),
        size: bytes.len(),
    }
}
fn leak<T>(value: T) -> &'static T {
    Box::leak(Box::new(value))
}

extern "C" fn company(company: CompanyID) -> *mut OpenTTDCompany {
    handle(usize::from(company.0))
}
extern "C" fn next_company(from: u32) -> *mut OpenTTDCompany {
    let count = COMPANIES.with(|r| r.borrow().len());
    if (from as usize) < count {
        handle(from as usize)
    } else {
        std::ptr::null_mut()
    }
}
extern "C" fn index(c: *mut OpenTTDCompany) -> CompanyID {
    CompanyID(u8::try_from(id(c)).unwrap())
}
extern "C" fn admin_update(c: *mut OpenTTDCompany) {
    trace(format!("admin {}", id(c)));
}
extern "C" fn build_owner_legend() {
    trace("BuildOwnerLegend");
}
extern "C" fn colour(c: *mut OpenTTDCompany) -> Colours {
    with(c, |r| r.colour)
}
extern "C" fn set_colour(c: *mut OpenTTDCompany, value: Colours) {
    trace(format!("colour {} {}", id(c), value.0));
    with(c, |r| r.colour = value);
}
extern "C" fn is_ai(c: *mut OpenTTDCompany) -> bool {
    with(c, |r| r.is_ai)
}
extern "C" fn last_build_coordinate(c: *mut OpenTTDCompany) -> TileIndex {
    with(c, |r| r.last_build_coordinate)
}
extern "C" fn livery(c: *mut OpenTTDCompany) -> *mut Livery {
    with(c, |r| r.livery.as_mut_ptr())
}
extern "C" fn name(c: *mut OpenTTDCompany) -> ByteSpan {
    with(c, |r| span(&r.name))
}
extern "C" fn set_name(c: *mut OpenTTDCompany, value: ByteSpan) {
    // SAFETY: The code under test passes live bytes.
    let value = unsafe { value.bytes() }.to_vec();
    trace(format!("name {} {}", id(c), text(&value)));
    with(c, |r| r.name = value);
}
extern "C" fn name_1(c: *mut OpenTTDCompany) -> StringID {
    with(c, |r| r.name_1)
}
extern "C" fn set_name_1(c: *mut OpenTTDCompany, value: StringID) {
    trace(format!("name_1 {} {}", id(c), value.0));
    with(c, |r| r.name_1 = value);
}
extern "C" fn name_2(c: *mut OpenTTDCompany) -> u32 {
    with(c, |r| r.name_2)
}
extern "C" fn set_name_2(c: *mut OpenTTDCompany, value: u32) {
    trace(format!("name_2 {} {value}", id(c)));
    with(c, |r| r.name_2 = value);
}
extern "C" fn president_name(c: *mut OpenTTDCompany) -> ByteSpan {
    with(c, |r| span(&r.president_name))
}
extern "C" fn set_president_name(c: *mut OpenTTDCompany, value: ByteSpan) {
    // SAFETY: The code under test passes live bytes.
    let value = unsafe { value.bytes() }.to_vec();
    trace(format!("president_name {} {}", id(c), text(&value)));
    with(c, |r| r.president_name = value);
}
extern "C" fn president_name_1(c: *mut OpenTTDCompany) -> StringID {
    with(c, |r| r.president_name_1)
}
extern "C" fn president_name_2(c: *mut OpenTTDCompany) -> u32 {
    with(c, |r| r.president_name_2)
}
extern "C" fn reset_colourmap() {
    trace("ResetVehicleColourMap");
}
extern "C" fn screen_dirty() {
    trace("MarkWholeScreenDirty");
}
extern "C" fn current_company() -> u8 {
    CURRENT.get()
}
extern "C" fn utf8_length(_: *const u8, size: usize) -> usize {
    size
}
extern "C" fn draw(_: *mut std::ffi::c_void) -> u32 {
    let draw = DRAWS.get();
    DRAWS.set(draw + 1);
    draw
}
extern "C" fn no_group(_: u32) -> u32 {
    u32::MAX
}

/// The fixture's `CommandCost`: success and the error message.
#[derive(Clone, Copy)]
struct FixtureCost {
    ok: bool,
    message: u32,
}
fn fixture_cost(cost: *const CommandCost) -> *mut FixtureCost {
    cost.cast::<FixtureCost>().cast_mut()
}
unsafe extern "C" fn construct(
    storage: *mut crate::services::CommandCostStorage,
) -> *mut CommandCost {
    let cost = storage.cast::<FixtureCost>();
    // SAFETY: The storage is the command's own aligned stack capacity.
    unsafe {
        cost.write(FixtureCost {
            ok: true,
            message: 0,
        });
    }
    cost.cast()
}
unsafe extern "C" fn destroy(_: *mut CommandCost) {}
extern "C" fn zero(cost: *mut CommandCost) {
    // SAFETY: A live fixture cost.
    unsafe { (*fixture_cost(cost)).ok = true };
}
extern "C" fn error(cost: *mut CommandCost, message: u32) {
    // SAFETY: A live fixture cost.
    unsafe {
        *fixture_cost(cost) = FixtureCost { ok: false, message };
    }
}
extern "C" fn success(cost: *const CommandCost) -> bool {
    // SAFETY: A live fixture cost.
    unsafe { (*fixture_cost(cost)).ok }
}
extern "C" fn money(_: *const CommandCost) -> i64 {
    0
}
extern "C" fn no_finances(_: u8) -> *mut crate::company::Finances {
    std::ptr::null_mut()
}

/// Record each window and news call in the trace as it is made.
fn window(effect: &Window) {
    trace(match *effect {
        Window::InvalidateWindowData(cls, number, data, gui_scope) => format!(
            "InvalidateWindowData {} {} {data} {gui_scope}",
            cls.0, number.0
        ),
        Window::InvalidateWindowClassesData(cls, data, gui_scope) => {
            format!("InvalidateWindowClassesData {} {data} {gui_scope}", cls.0)
        }
        ref effect => panic!("unexpected window call {effect:?}"),
    });
}
fn news(effect: &News) {
    trace(match effect {
        News::GetString(string, params) => format!("GetString {string} {params:?}"),
        News::AiBroadcastNewTextEvent(event, name, skip) => format!(
            "AI {} {} {} {skip}",
            event.r#type.0,
            event.company,
            text(name)
        ),
        News::GameNewTextEvent(event, name) => {
            format!("Game {} {} {}", event.r#type.0, event.company, text(name))
        }
        News::CompanyNewsInformation(title, company, other) => {
            format!("CompanyNewsInformation {title} {company} {other}")
        }
        News::News { .. } => "AddNewsItem".to_owned(),
        effect => panic!("unexpected news call {effect:?}"),
    });
}

/// The identity table over `companies`, with `current` the current company.
fn services(companies: Vec<Record>, current: u8) -> &'static Services {
    crate::news_func::testing::take();
    crate::window_func::testing::take();
    crate::news_func::testing::observe(Some(news));
    crate::window_func::testing::observe(Some(window));
    COMPANIES.with(|r| *r.borrow_mut() = companies);
    TRACE.with(|t| t.borrow_mut().clear());
    CURRENT.set(current);
    DRAWS.set(0);
    let mut shared = SHARED;
    shared.current_company = current_company;
    shared.screen_dirty = screen_dirty;
    shared.utf8_length = utf8_length;
    shared.random = draw;
    shared.vehicle_next = vehicle_next;
    shared.map = crate::map_func::fixture::statics();
    shared.town_index = map_town_index;
    shared.town_by_id = town_by_id;
    shared.town_owner = town_owner;
    let shared = leak(shared);
    let mut costs = crate::test_tables::COMMAND_COSTS;
    costs.construct = construct;
    costs.destroy = destroy;
    costs.zero = zero;
    costs.error = error;
    costs.success = success;
    costs.money = money;
    let mut finance = FINANCE;
    finance.current_company = current_company;
    finance.company_finances = no_finances;
    let mut commands = COMMANDS;
    commands.shared = shared;
    commands.cost = leak(costs);
    commands.finance = leak(finance);
    let mut groups = FLEET_GROUPS;
    groups.next_group = no_group;
    let mut s = COMPANY_IDENTITY;
    s.shared = shared;
    s.commands = leak(commands);
    s.groups = leak(groups);
    s.capabilities = capabilities(shared);
    s.authority = leak(crate::town::AuthorityServices {
        bulldozer: crate::test_tables::unreachable0,
        town_index: authority_town_index,
    });
    s.admin_update = admin_update;
    s.build_owner_legend = build_owner_legend;
    s.colour = colour;
    s.company = company;
    s.index = index;
    s.is_ai = is_ai;
    s.last_build_coordinate = last_build_coordinate;
    s.livery = livery;
    s.name = name;
    s.name_1 = name_1;
    s.name_2 = name_2;
    s.next_company = next_company;
    s.objects = object_services;
    s.president_name = president_name;
    s.president_name_1 = president_name_1;
    s.president_name_2 = president_name_2;
    s.reset_colourmap = reset_colourmap;
    s.set_colour = set_colour;
    s.set_name = set_name;
    s.set_name_1 = set_name_1;
    s.set_name_2 = set_name_2;
    s.set_president_name = set_president_name;
    leak(s)
}
fn take_trace() -> Vec<std::string::String> {
    TRACE.with(|t| std::mem::take(&mut *t.borrow_mut()))
}
/// End the recorders' observation; return the recorded news effects.
fn finish() -> Vec<News> {
    crate::news_func::testing::observe(None);
    crate::window_func::testing::observe(None);
    crate::window_func::testing::take();
    crate::news_func::testing::take()
}

/// Run `command` as `CommandHelper::Do` runs it at the top level, executing.
fn run(s: &Services, command: impl Fn(Cost<'_>, DoCommandFlags)) -> FixtureCost {
    // SAFETY: The result lives on this stack for the synchronous command.
    unsafe {
        crate::command::with_cost(s.commands.cost, |out| {
            crate::command::nested(
                s.commands,
                s.shared,
                out,
                0,
                DoCommandFlag::EXECUTE,
                |flags| command(out, DoCommandFlags(flags)),
            );
            *fixture_cost(out.0)
        })
    }
}

/// `CmdRenamePresident` for a company with `name_1 == STR_SV_UNNAMED` and no name: the
/// president store, the nested `CMD_RENAME_COMPANY` of the text and " Transport" with its
/// own effects, then the company window classes, the screen, the admin update and the
/// rendered `STR_PRESIDENT_NAME` in both renamed events.
#[test]
fn rename_president_renames_an_unnamed_company() {
    let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
    let mut other = Record::new(Colours::COLOUR_RED);
    other.president_name = b"Bob".to_vec();
    let s = services(vec![Record::new(Colours::COLOUR_BLUE), other], 0);
    let result = run(s, |out, flags| {
        cmd_rename_president(
            s,
            out,
            flags,
            CmdRenamePresident {
                text: span(b"Alice"),
            },
        );
    });
    assert!(result.ok);
    let company = STR_COMPANY_NAME.0;
    let president = STR_PRESIDENT_NAME.0;
    assert_eq!(
        take_trace(),
        [
            "president_name 0 Alice".to_owned(),
            "name 0 Alice Transport".to_owned(),
            "MarkWholeScreenDirty".to_owned(),
            "admin 0".to_owned(),
            format!("GetString {company} [0]"),
            format!("AI 33 0 S{company}:0 255"),
            format!("Game 33 0 S{company}:0"),
            format!(
                "InvalidateWindowClassesData {} 1 false",
                WindowClass::WC_COMPANY.0
            ),
            "MarkWholeScreenDirty".to_owned(),
            "admin 0".to_owned(),
            format!("GetString {president} [0]"),
            format!("AI 34 0 S{president}:0 255"),
            format!("Game 34 0 S{president}:0"),
        ]
    );
    finish();
}

/// An empty text resets the president's name and starts no nested rename; a taken name
/// fails with `STR_ERROR_NAME_MUST_BE_UNIQUE` before any effect.
#[test]
fn rename_president_resets_and_refuses_a_taken_name() {
    let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
    let mut other = Record::new(Colours::COLOUR_RED);
    other.president_name = b"Bob".to_vec();
    let s = services(vec![Record::new(Colours::COLOUR_BLUE), other], 0);
    let result = run(s, |out, flags| {
        cmd_rename_president(s, out, flags, CmdRenamePresident { text: span(b"") });
    });
    assert!(result.ok);
    let president = STR_PRESIDENT_NAME.0;
    assert_eq!(
        take_trace(),
        [
            "president_name 0 ".to_owned(),
            format!(
                "InvalidateWindowClassesData {} 1 false",
                WindowClass::WC_COMPANY.0
            ),
            "MarkWholeScreenDirty".to_owned(),
            "admin 0".to_owned(),
            format!("GetString {president} [0]"),
            format!("AI 34 0 S{president}:0 255"),
            format!("Game 34 0 S{president}:0"),
        ]
    );
    let result = run(s, |out, flags| {
        cmd_rename_president(s, out, flags, CmdRenamePresident { text: span(b"Bob") });
    });
    assert!(!result.ok);
    assert_eq!(result.message, STR_ERROR_NAME_MUST_BE_UNIQUE.0);
    assert_eq!(take_trace(), Vec::<std::string::String>::new());
    finish();
}

thread_local! {
    static CACHES: RefCell<Vec<crate::newgrf_vehicle::Cache>> = const { RefCell::new(Vec::new()) };
}
extern "C" fn vehicle_next(from: u32) -> Option<VehicleRef> {
    VEHICLES.with(|v| v.borrow().get(from as usize).copied())
}
unsafe extern "C" fn vehicle_id(v: *mut OpenTTDVehicle) -> u32 {
    VEHICLES.with(|list| {
        u32::try_from(
            list.borrow()
                .iter()
                .position(|r| r.0.as_ptr() == v)
                .unwrap(),
        )
        .unwrap()
    })
}
unsafe extern "C" fn vehicle_cache(v: *mut OpenTTDVehicle) -> *mut crate::newgrf_vehicle::Cache {
    // SAFETY: The fixture's own vehicle.
    let index = unsafe { vehicle_id(v) };
    trace(format!("InvalidateNewGRFCache {index}"));
    CACHES.with(|c| c.borrow_mut().as_mut_ptr().wrapping_add(index as usize))
}
extern "C" fn newgrf() -> *const crate::newgrf_vehicle::Services {
    let mut resolver = crate::newgrf_vehicle::TEST_TABLE;
    resolver.id = vehicle_id;
    resolver.cache = vehicle_cache;
    leak(resolver)
}
fn capabilities(
    shared: &'static crate::services::Services,
) -> &'static crate::engine_capabilities::Services {
    let mut vehicles = crate::test_tables::VEHICLE;
    vehicles.newgrf = newgrf;
    let mut capabilities = crate::engine_capabilities::test_table_with_shared(shared);
    capabilities.vehicles = leak(vehicles);
    leak(capabilities)
}
unsafe extern "C" fn no_object(_: u32) -> crate::objects::Next {
    trace("UpdateObjectColours");
    crate::objects::Next {
        object: std::ptr::null_mut(),
        id: 0,
        tile: 0,
    }
}
extern "C" fn object_services() -> *const crate::objects::ObjectServices {
    let mut objects = crate::test_tables::OBJECTS;
    objects.shared = &SHARED;
    objects.next = no_object;
    leak(objects)
}
fn vehicles(owners: &[CompanyID]) {
    let records = owners
        .iter()
        .map(|&owner| {
            let record = crate::vehicle_base::VehicleRecord::new(
                std::ptr::null_mut(),
                crate::vehicle_type::VehicleType::VEH_TRAIN,
            );
            // SAFETY: The fixture's own new record.
            unsafe { (*record).owner = owner };
            VehicleRef(std::ptr::NonNull::new(record.cast()).unwrap())
        })
        .collect();
    VEHICLES.with(|v| *v.borrow_mut() = records);
    CACHES.with(|c| {
        *c.borrow_mut() = (0..owners.len())
            .map(|_| crate::newgrf_vehicle::Cache::default())
            .collect();
    });
}

/// `CmdSetCompanyColour` of the default scheme's primary colour: the company colour and its
/// cache, the admin update, `ResetVehicleColourMap`, the screen, the six graph windows with
/// number and data 0, `BuildOwnerLegend` and `WC_SMALLMAP` with data 1, in the original's
/// order; then only the current company's vehicles lose their cache, before the objects.
#[test]
fn set_company_colour_invalidates_in_order() {
    let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
    let s = services(
        vec![
            Record::new(Colours::COLOUR_BLUE),
            Record::new(Colours::COLOUR_RED),
        ],
        1,
    );
    vehicles(&[CompanyID(0), CompanyID(1), CompanyID(1)]);
    let saved = crate::company_identity::openttd_rust_company_colours().wrapping_add(1);
    // SAFETY: The process-wide colour cache, serialized by the lock and restored below.
    let earlier = unsafe { saved.read() };
    let result = run(s, |out, flags| {
        cmd_set_company_colour(
            s,
            out,
            flags,
            CmdSetCompanyColour {
                scheme: LiveryScheme::LS_DEFAULT,
                primary: true,
                colour: Colours::COLOUR_GREEN,
            },
        );
    });
    assert!(result.ok);
    // SAFETY: As above.
    assert_eq!(unsafe { saved.read() }, Colours::COLOUR_GREEN.0);
    // SAFETY: As above.
    unsafe { saved.write(earlier) };
    let graph = |cls: WindowClass| format!("InvalidateWindowData {} 0 0 false", cls.0);
    assert_eq!(
        take_trace(),
        [
            format!("colour 1 {}", Colours::COLOUR_GREEN.0),
            "admin 1".to_owned(),
            "ResetVehicleColourMap".to_owned(),
            "MarkWholeScreenDirty".to_owned(),
            graph(WindowClass::WC_INCOME_GRAPH),
            graph(WindowClass::WC_OPERATING_PROFIT),
            graph(WindowClass::WC_DELIVERED_CARGO),
            graph(WindowClass::WC_PERFORMANCE_HISTORY),
            graph(WindowClass::WC_COMPANY_VALUE),
            graph(WindowClass::WC_LINKGRAPH_LEGEND),
            "BuildOwnerLegend".to_owned(),
            format!(
                "InvalidateWindowData {} 0 1 false",
                WindowClass::WC_SMALLMAP.0
            ),
            "InvalidateNewGRFCache 1".to_owned(),
            "InvalidateNewGRFCache 2".to_owned(),
            "UpdateObjectColours".to_owned(),
        ]
    );
    // The default scheme's primary colour; the colour of a scheme without its own follows.
    COMPANIES.with(|r| {
        let r = &r.borrow()[1];
        assert_eq!(r.livery[0].colour1, Colours::COLOUR_GREEN);
        assert_eq!(r.livery[5].colour1, Colours::COLOUR_GREEN);
        assert_eq!(r.livery[0].in_use, LiveryFlags::EMPTY);
    });
    // Another company's colour is refused before any effect.
    let result = run(s, |out, flags| {
        cmd_set_company_colour(
            s,
            out,
            flags,
            CmdSetCompanyColour {
                scheme: LiveryScheme::LS_DEFAULT,
                primary: true,
                colour: Colours::COLOUR_BLUE,
            },
        );
    });
    assert!(!result.ok);
    assert_eq!(take_trace(), Vec::<std::string::String>::new());
    finish();
}

extern "C" fn map_town_index(tile: u32) -> u16 {
    assert_eq!(tile, 5);
    3
}
extern "C" fn town_by_id(id: u32) -> *mut crate::services::TownHandle {
    assert_eq!(id, 3);
    std::ptr::without_provenance_mut(64)
}
extern "C" fn town_owner(t: *const crate::services::TownHandle) -> *mut crate::town::State {
    assert_eq!(t.addr(), 64);
    TOWN.with(|t| t.get())
}
extern "C" fn authority_town_index(t: *const crate::services::TownHandle) -> u16 {
    assert_eq!(t.addr(), 64);
    3
}
thread_local! {
    static TOWN: Cell<*mut crate::town::State> = const { Cell::new(std::ptr::null_mut()) };
}
/// A 64x64 map whose tile 5 is a house of town 3, named by the town name generator 7.
fn house_map() -> Vec<crate::map_func::BaseTileLayout> {
    let mut base: Vec<_> = (0..4096)
        .map(|_| crate::map_func::BaseTileLayout {
            kind: 0,
            height: 0,
            m2: 0,
            m1: 0,
            m3: 0,
            m4: 0,
            m5: 0,
        })
        .collect();
    base[5].kind = crate::tile_type::TileType::MP_HOUSE.0 << 4;
    crate::map_func::fixture::set(
        crate::map_func::TileTypeAllocation {
            base: base.as_ptr(),
            extended: std::ptr::null(),
            count: 4096,
            assertions: true,
        },
        6,
    );
    let mut town = Box::new(crate::town::State::default());
    town.simulation.townnametype = u16::try_from(SPECSTR_TOWNNAME_START.0 + 7).unwrap();
    town.simulation.townnameparts = 0x55;
    TOWN.set(Box::into_raw(town));
    base
}

/// `GenerateCompanyName` names an AI company after its closest town and launches its news:
/// `NewsType::CompanyInfo`, `NewsStyle::Company`, no flags, a tile reference to the last build
/// coordinate and the company's `CompanyNewsInformation` with a headline of the company name
/// and the town. A human company gets no news.
#[test]
fn generate_company_name_launches_an_ai_company() {
    let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
    let _map = house_map();
    for ai in [true, false] {
        let mut record = Record::new(Colours::COLOUR_BLUE);
        record.is_ai = ai;
        record.last_build_coordinate = TileIndex(5);
        let s = services(vec![record], 0);
        generate_company_name(s, handle(0));
        let str = SPECSTR_COMPANY_NAME_START.0 + 7;
        let mut expected = vec![
            format!("GetString {str} [85]"),
            format!("name_1 0 {str}"),
            "name_2 0 85".to_owned(),
            "MarkWholeScreenDirty".to_owned(),
            format!("AI 33 0 S{str}:85 255"),
            format!("Game 33 0 S{str}:85"),
        ];
        if ai {
            expected.extend([
                format!(
                    "CompanyNewsInformation {} 0 {}",
                    STR_NEWS_COMPANY_LAUNCH_TITLE.0,
                    CompanyID::INVALID.0
                ),
                "AddNewsItem".to_owned(),
            ]);
        }
        assert_eq!(take_trace(), expected, "ai {ai}");
        let effects = finish();
        let news: Vec<_> = effects
            .into_iter()
            .filter(|e| matches!(e, News::News { .. }))
            .collect();
        let name = crate::news_func::testing::company_news_name(0);
        let expected: Vec<News> = if ai {
            vec![News::News {
                headline: encoded(
                    STR_NEWS_COMPANY_LAUNCH_DESCRIPTION.0,
                    &[Param::S(&name), Param::N(3)],
                ),
                r#type: NewsType::CompanyInfo,
                style: NewsStyle::Company,
                flags: NewsFlags::default(),
                ref1: NewsReference::Tile(TileIndex(5)),
                ref2: NewsReference::None,
                data: Some(name.clone()),
                advice_type: crate::news_type::AdviceType::Invalid,
            }]
        } else {
            Vec::new()
        };
        assert_eq!(news, expected, "ai {ai}");
    }
    // SAFETY: Allocated by `house_map`.
    drop(unsafe { Box::from_raw(TOWN.replace(std::ptr::null_mut())) });
}

extern "C" fn no_spec(_: u32) -> *const crate::company_manager_face::FaceSpec {
    std::ptr::null()
}
extern "C" fn stored(_: *mut CompanyManagerFace, _: u32) {
    panic!("the style is stored after the assertion");
}

/// `SetCompanyManagerFaceStyle` asserts its spec before it stores the style: a null spec
/// ends at the native assertion, not at the label store.
#[test]
fn set_company_manager_face_style_asserts_its_spec() {
    if crate::services_assert::isolate_test_exit(
        "company_identity::tests::set_company_manager_face_style_asserts_its_spec",
    ) {
        return;
    }
    let mut faces = crate::test_tables::FACES;
    faces.spec = no_spec;
    faces.set_style = stored;
    set_company_manager_face_style(&faces, std::ptr::null_mut(), 4);
}
