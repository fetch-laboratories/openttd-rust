/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Goals, story pages and league command/lifecycle policy. Native pools and strings retain storage.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation
)]
use crate::command_type::DoCommandFlag;
use crate::story_base::{
    SPET_BUTTON_PUSH, SPET_BUTTON_TILE, SPET_BUTTON_VEHICLE, SPET_GOAL, SPET_LOCATION, SPET_TEXT,
    StoryPageButtonData,
};
use crate::tile_type::TileIndex;
use crate::vehicle_type::VehicleID;
use crate::window_type::{
    WC_COMPANY_LEAGUE, WC_GOAL_QUESTION, WC_GOALS_LIST, WC_MAIN_TOOLBAR, WC_STORY_BOOK,
};
use crate::{
    company_type::CompanyID,
    goal_type::{
        GOAL_QUESTION_BUTTON_COUNT, GQT_END, GQT_QUESTION, GT_COMPANY, GT_INDUSTRY, GT_NONE,
        GT_STORY_PAGE, GT_TILE, GT_TOWN, Goal, GoalID,
    },
    league_type::{
        LT_COMPANY, LT_INDUSTRY, LT_NONE, LT_STORY_PAGE, LT_TILE, LT_TOWN, LeagueTableElement,
        LeagueTableElementID, LeagueTableID,
    },
    story_type::{StoryPage, StoryPageElement, StoryPageElementID, StoryPageID},
};
#[repr(C)]
pub struct EncodedString {
    _private: [u8; 0],
}
#[repr(C)]
pub struct Services {
    pub shared: *const crate::services::Services,
    pub company: *const crate::company::Services,
    pub map_allocation: extern "C" fn() -> crate::map_func::TileTypeAllocation,
    pub goal: extern "C" fn(GoalID) -> *mut Goal,
    pub goal_company: unsafe extern "C" fn(*mut Goal) -> *mut CompanyID,
    pub goal_type: unsafe extern "C" fn(*mut Goal) -> *mut u8,
    pub goal_dst: unsafe extern "C" fn(*mut Goal) -> *mut u32,
    pub goal_text: unsafe extern "C" fn(*mut Goal) -> *mut EncodedString,
    pub goal_progress: unsafe extern "C" fn(*mut Goal) -> *mut EncodedString,
    pub goal_completed: unsafe extern "C" fn(*mut Goal) -> *mut bool,
    pub goal_count: extern "C" fn() -> u32,
    pub goal_can_allocate: extern "C" fn() -> bool,
    pub page: extern "C" fn(u32) -> *mut StoryPage,
    pub page_company: unsafe extern "C" fn(*mut StoryPage) -> *mut CompanyID,
    pub page_title: unsafe extern "C" fn(*mut StoryPage) -> *mut EncodedString,
    pub page_date: unsafe extern "C" fn(*mut StoryPage) -> *mut i32,
    pub page_count: extern "C" fn() -> u32,
    pub page_can_allocate: extern "C" fn() -> bool,
    pub element: extern "C" fn(StoryPageElementID) -> *mut StoryPageElement,
    pub element_page: unsafe extern "C" fn(*mut StoryPageElement) -> *mut StoryPageID,
    pub element_type: unsafe extern "C" fn(*mut StoryPageElement) -> *mut u8,
    pub element_text: unsafe extern "C" fn(*mut StoryPageElement) -> *mut EncodedString,
    pub element_referenced_id: unsafe extern "C" fn(*mut StoryPageElement) -> *mut u32,
    pub element_destroy: unsafe extern "C" fn(*mut StoryPageElement),
    pub element_count: extern "C" fn() -> u32,
    pub element_can_allocate: extern "C" fn() -> bool,
    pub entry: extern "C" fn(LeagueTableElementID) -> *mut LeagueTableElement,
    pub entry_company: unsafe extern "C" fn(*mut LeagueTableElement) -> *mut CompanyID,
    pub entry_table: unsafe extern "C" fn(*mut LeagueTableElement) -> *mut LeagueTableID,
    pub entry_rating: unsafe extern "C" fn(*mut LeagueTableElement) -> *mut i64,
    pub entry_text: unsafe extern "C" fn(*mut LeagueTableElement) -> *mut EncodedString,
    pub entry_score: unsafe extern "C" fn(*mut LeagueTableElement) -> *mut EncodedString,
    pub entry_link_type: unsafe extern "C" fn(*mut LeagueTableElement) -> *mut u8,
    pub entry_link_target: unsafe extern "C" fn(*mut LeagueTableElement) -> *mut u32,
    pub entry_destroy: unsafe extern "C" fn(*mut LeagueTableElement),
    pub entry_can_allocate: extern "C" fn() -> bool,
    pub goal_allocate: unsafe extern "C" fn(u8, u32, CompanyID, *const EncodedString) -> GoalID,
    pub page_allocate:
        unsafe extern "C" fn(u32, i32, CompanyID, *const EncodedString) -> StoryPageID,
    pub element_allocate: extern "C" fn(u32, u8, StoryPageID) -> StoryPageElementID,
    pub element_next: extern "C" fn(u32) -> *mut StoryPageElement,
    pub element_id: unsafe extern "C" fn(*mut StoryPageElement) -> StoryPageElementID,
    pub page_cleaning: extern "C" fn() -> bool,
    pub table_can_allocate: extern "C" fn() -> bool,
    pub table_allocate: unsafe extern "C" fn(
        *const EncodedString,
        *const EncodedString,
        *const EncodedString,
    ) -> LeagueTableID,
    pub entry_allocate: unsafe extern "C" fn(
        LeagueTableID,
        i64,
        CompanyID,
        *const EncodedString,
        *const EncodedString,
        u8,
        u32,
    ) -> LeagueTableElementID,
    pub orders: *const crate::orders::Leaves,
    pub client_valid: extern "C" fn(u32) -> bool,
    pub subsidy: *const crate::subsidy::Services,
    pub road_commands: *const crate::road_cmd::RoadCommandServices,
    pub vehicle_valid: extern "C" fn(VehicleID) -> bool,
    pub text_empty: unsafe extern "C" fn(*const EncodedString) -> bool,
    pub text_assign: unsafe extern "C" fn(*mut EncodedString, *const EncodedString),
    pub window_data: extern "C" fn(u16, u32, i32),
    pub window_classes: extern "C" fn(u16, i32),
    pub window_close: extern "C" fn(u16, u32),
    pub question_show: unsafe extern "C" fn(u16, u8, u32, *const EncodedString),
    pub story_show: extern "C" fn(CompanyID, StoryPageID),
    pub question_event: extern "C" fn(u16, CompanyID, u32),
    pub push_event: extern "C" fn(CompanyID, StoryPageID, StoryPageElementID),
    pub tile_event: extern "C" fn(CompanyID, StoryPageID, StoryPageElementID, TileIndex),
    pub vehicle_event: extern "C" fn(CompanyID, StoryPageID, StoryPageElementID, VehicleID),
}
impl Services {
    fn shared(&self) -> &crate::services::Services {
        unsafe { &*self.shared }
    }
    fn company(&self) -> &crate::company::Services {
        unsafe { &*self.company }
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDStoryState" ["probe"] => StoryState { page_next_sort, element_next_sort };
    "OpenTTDObjectivesServices" ["probe"] => Services { shared, company, map_allocation, goal, goal_company, goal_type, goal_dst, goal_text, goal_progress, goal_completed, goal_count, goal_can_allocate, page, page_company, page_title, page_date, page_count, page_can_allocate, element, element_page, element_type, element_text, element_referenced_id, element_destroy, element_count, element_can_allocate, entry, entry_company, entry_table, entry_rating, entry_text, entry_score, entry_link_type, entry_link_target, entry_destroy, entry_can_allocate, goal_allocate, page_allocate, element_allocate, element_next, element_id, page_cleaning, table_can_allocate, table_allocate, entry_allocate, orders, client_valid, subsidy, road_commands, vehicle_valid, text_empty, text_assign, window_data, window_classes, window_close, question_show, story_show, question_event, push_event, tile_event, vehicle_event };
}

// Native loader aliases and Rust control share one process-wide allocation. Accesses
// hold the game-state mutex; raw field loads/stores end before every callback.
#[repr(C)]
pub struct StoryState {
    pub page_next_sort: u32,
    pub element_next_sort: u32,
}
static STORY: crate::game_state::GameState<std::mem::MaybeUninit<StoryState>> =
    crate::game_state::GameState::new(std::mem::MaybeUninit::zeroed());
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_story_storage() -> *mut StoryState {
    STORY.get().cast()
}
fn page_sort() -> *mut u32 {
    unsafe { std::ptr::addr_of_mut!((*STORY.get().cast::<StoryState>()).page_next_sort) }
}
fn element_sort() -> *mut u32 {
    unsafe { std::ptr::addr_of_mut!((*STORY.get().cast::<StoryState>()).element_next_sort) }
}
fn execute(flags: u16) -> bool {
    flags & DoCommandFlag::EXECUTE != 0
}
impl Services {
    fn deity(&self) -> bool {
        (self.shared().current_company)() == CompanyID::DEITY.0
    }
    fn company_valid(&self, company: CompanyID) -> bool {
        (self.shared().company_valid)(company.0)
    }
    fn industry_valid(&self, id: u32) -> bool {
        u16::try_from(id).is_ok_and(|id| unsafe {
            ((*self.subsidy).valid_source)(crate::source_type::Source::industry(id))
        })
    }
    fn town_valid(&self, id: u32) -> bool {
        u16::try_from(id).is_ok_and(|id| unsafe { ((*self.road_commands).town_valid)(id) })
    }
    fn tile_valid(&self, tile: TileIndex) -> bool {
        let allocation = (self.map_allocation)();
        // SAFETY: command/lifecycle execution holds the mutex and cannot reset or yield the map.
        let map =
            unsafe { crate::map_func::TileTypeAccess::raw(allocation, self.shared().tile_type) };
        crate::tile_map::is_valid_tile(&map, tile.0, allocation.count).unwrap()
    }
    fn goal_dirty(&self, company: CompanyID) {
        if company == CompanyID::INVALID {
            (self.window_classes)(WC_GOALS_LIST, 0);
        } else {
            (self.window_data)(WC_GOALS_LIST, u32::from(company.0), 0);
        }
    }
    fn toolbar_dirty(&self) {
        (self.window_data)(WC_MAIN_TOOLBAR, 0, 0);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_valid_goal_destination(
    s: &Services,
    company: CompanyID,
    kind: u8,
    dest: u32,
) -> bool {
    match kind {
        GT_NONE => dest == 0,
        GT_TILE => s.tile_valid(TileIndex(dest)),
        GT_INDUSTRY => s.industry_valid(dest),
        GT_TOWN => s.town_valid(dest),
        // Company::IsValidID takes the full u32; IDs above u8 lie past the company pool.
        GT_COMPANY => u8::try_from(dest).is_ok_and(|id| s.company_valid(CompanyID(id))),
        GT_STORY_PAGE => {
            let page = (s.page)(dest);
            if page.is_null() {
                return false;
            }
            let owner = unsafe { *(s.page_company)(page) };
            if company == CompanyID::INVALID {
                owner == CompanyID::INVALID
            } else {
                owner == CompanyID::INVALID || owner == company
            }
        }
        _ => false,
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_create_goal(
    s: &Services,
    flags: u16,
    company: CompanyID,
    kind: u8,
    dest: u32,
    text: *const EncodedString,
    result: *mut GoalID,
) -> bool {
    unsafe {
        result.write(GoalID::INVALID);
        if !(s.goal_can_allocate)()
            || !s.deity()
            || (s.text_empty)(text)
            || (company != CompanyID::INVALID && !s.company_valid(company))
            || !openttd_rust_valid_goal_destination(s, company, kind, dest)
        {
            return false;
        }
        if execute(flags) {
            let id = (s.goal_allocate)(kind, dest, company, text);
            let goal = (s.goal)(id);
            s.goal_dirty(*(s.goal_company)(goal));
            if (s.goal_count)() == 1 {
                s.toolbar_dirty();
            }
            result.write(id);
        }
        true
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_remove_goal(s: &Services, flags: u16, id: GoalID) -> bool {
    unsafe {
        if !s.deity() {
            return false;
        }
        let goal = (s.goal)(id);
        if goal.is_null() {
            return false;
        }
        if execute(flags) {
            let company = *(s.goal_company)(goal);
            (s.company().delete_goal)(u32::from(id.0));
            s.goal_dirty(company);
            if (s.goal_count)() == 0 {
                s.toolbar_dirty();
            }
        }
        true
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_set_goal_destination(
    s: &Services,
    flags: u16,
    id: GoalID,
    kind: u8,
    dest: u32,
) -> bool {
    unsafe {
        if !s.deity() {
            return false;
        }
        let goal = (s.goal)(id);
        if goal.is_null() {
            return false;
        }
        if !openttd_rust_valid_goal_destination(s, *(s.goal_company)(goal), kind, dest) {
            return false;
        }
        if execute(flags) {
            (s.goal_type)(goal).write(kind);
            (s.goal_dst)(goal).write(dest);
        }
        true
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_set_goal_text(
    s: &Services,
    flags: u16,
    id: GoalID,
    text: *const EncodedString,
) -> bool {
    unsafe {
        if !s.deity() {
            return false;
        }
        let goal = (s.goal)(id);
        if goal.is_null() || (s.text_empty)(text) {
            return false;
        }
        if execute(flags) {
            (s.text_assign)((s.goal_text)(goal), text);
            s.goal_dirty(*(s.goal_company)(goal));
        }
        true
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_set_goal_progress(
    s: &Services,
    flags: u16,
    id: GoalID,
    text: *const EncodedString,
) -> bool {
    unsafe {
        if !s.deity() {
            return false;
        }
        let goal = (s.goal)(id);
        if goal.is_null() {
            return false;
        }
        if execute(flags) {
            (s.text_assign)((s.goal_progress)(goal), text);
            s.goal_dirty(*(s.goal_company)(goal));
        }
        true
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_set_goal_completed(
    s: &Services,
    flags: u16,
    id: GoalID,
    completed: bool,
) -> bool {
    unsafe {
        if !s.deity() {
            return false;
        }
        let goal = (s.goal)(id);
        if goal.is_null() {
            return false;
        }
        if execute(flags) {
            (s.goal_completed)(goal).write(completed);
            s.goal_dirty(*(s.goal_company)(goal));
        }
        true
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_goal_question(
    s: &Services,
    flags: u16,
    id: u16,
    target: u32,
    is_client: bool,
    mut buttons: u32,
    kind: u8,
    text: *const EncodedString,
) -> bool {
    let company = CompanyID(target as u8);
    buttons &= (1_u32 << GOAL_QUESTION_BUTTON_COUNT) - 1;
    if !s.deity() || unsafe { (s.text_empty)(text) } {
        return false;
    }
    if is_client {
        if !execute(flags) && (s.company().network_server)() && !(s.client_valid)(target) {
            return false;
        }
    } else if company != CompanyID::INVALID && !s.company_valid(company) {
        return false;
    }
    let min_buttons = u32::from(kind == GQT_QUESTION);
    if buttons.count_ones() < min_buttons || buttons.count_ones() > 3 || kind >= GQT_END {
        return false;
    }
    if execute(flags) {
        if is_client {
            if target != unsafe { ((*s.orders).network_client)() as u32 } {
                return true;
            }
        } else {
            let local = CompanyID((s.company().local_company)());
            if company == CompanyID::INVALID && !s.company_valid(local) {
                return true;
            }
            if company != CompanyID::INVALID && company != local {
                return true;
            }
        }
        unsafe {
            (s.question_show)(id, kind, buttons, text);
        }
    }
    true
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_goal_question_answer(
    s: &Services,
    flags: u16,
    id: u16,
    button: u8,
) -> bool {
    if button >= GOAL_QUESTION_BUTTON_COUNT {
        return false;
    }
    if s.deity() {
        if execute(flags) {
            (s.window_close)(WC_GOAL_QUESTION, u32::from(id));
        }
        return true;
    }
    if (s.company().finance.networking)()
        && (s.company().local_company)() == (s.shared().current_company)()
    {
        if execute(flags) {
            (s.window_close)(WC_GOAL_QUESTION, u32::from(id));
        }
        if !(s.company().network_server)() {
            return true;
        }
    }
    if execute(flags) {
        (s.question_event)(id, CompanyID((s.shared().current_company)()), 1 << button);
    }
    true
}
unsafe fn verify_element(
    s: &Services,
    page: StoryPageID,
    kind: u8,
    tile: TileIndex,
    reference: u32,
    text: *const EncodedString,
) -> bool {
    let button = StoryPageButtonData(reference);
    unsafe {
        match kind {
            SPET_TEXT => !(s.text_empty)(text),
            SPET_LOCATION => !(s.text_empty)(text) && s.tile_valid(tile),
            SPET_GOAL => {
                let goal = (s.goal)(GoalID(reference as u16));
                if goal.is_null() {
                    return false;
                }
                !(*(s.page_company)((s.page)(u32::from(page.0))) == CompanyID::INVALID
                    && *(s.goal_company)(goal) != CompanyID::INVALID)
            }
            SPET_BUTTON_PUSH => button.validate_colour() && button.validate_flags(),
            SPET_BUTTON_TILE => {
                button.validate_colour() && button.validate_flags() && button.validate_cursor()
            }
            SPET_BUTTON_VEHICLE => {
                button.validate_colour()
                    && button.validate_flags()
                    && button.validate_cursor()
                    && button.validate_vehicle_type()
            }
            _ => false,
        }
    }
}
unsafe fn update_element(
    s: &Services,
    element: *mut StoryPageElement,
    tile: TileIndex,
    reference: u32,
    text: *const EncodedString,
) {
    unsafe {
        match *(s.element_type)(element) {
            SPET_TEXT => (s.text_assign)((s.element_text)(element), text),
            SPET_LOCATION => {
                (s.text_assign)((s.element_text)(element), text);
                (s.element_referenced_id)(element).write(tile.0);
            }
            SPET_GOAL => (s.element_referenced_id)(element).write(reference),
            SPET_BUTTON_PUSH | SPET_BUTTON_TILE | SPET_BUTTON_VEHICLE => {
                (s.text_assign)((s.element_text)(element), text);
                (s.element_referenced_id)(element).write(reference);
            }
            _ => unreachable!(),
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_create_story_page(
    s: &Services,
    flags: u16,
    company: CompanyID,
    text: *const EncodedString,
    result: *mut StoryPageID,
) -> bool {
    unsafe {
        result.write(StoryPageID::INVALID);
        if !(s.page_can_allocate)()
            || !s.deity()
            || (company != CompanyID::INVALID && !s.company_valid(company))
        {
            return false;
        }
        if execute(flags) {
            if (s.page_count)() == 0 {
                page_sort().write(0);
            }
            let id = (s.page_allocate)(
                *page_sort(),
                crate::timer_game_common::calendar_date().0,
                company,
                text,
            );
            (s.window_classes)(WC_STORY_BOOK, -1);
            if (s.page_count)() == 1 {
                s.toolbar_dirty();
            }
            let next = page_sort();
            next.write((*next).wrapping_add(1));
            result.write(id);
        }
        true
    }
}
unsafe fn each_element(s: &Services, mut visit: impl FnMut(*mut StoryPageElement)) {
    let mut from = 0;
    loop {
        let element = (s.element_next)(from);
        if element.is_null() {
            break;
        }
        from = u32::from(unsafe { (s.element_id)(element) }.0) + 1;
        visit(element);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_create_story_page_element(
    s: &Services,
    flags: u16,
    tile: TileIndex,
    page: StoryPageID,
    kind: u8,
    reference: u32,
    text: *const EncodedString,
    result: *mut StoryPageElementID,
) -> bool {
    unsafe {
        result.write(StoryPageElementID::INVALID);
        if !(s.element_can_allocate)() {
            return false;
        }
        let mut count = 0_u16;
        each_element(s, |element| {
            if *(s.element_page)(element) == page {
                count = count.wrapping_add(1);
            }
        });
        if count >= 128
            || !s.deity()
            || (s.page)(u32::from(page.0)).is_null()
            || !verify_element(s, page, kind, tile, reference, text)
        {
            return false;
        }
        if execute(flags) {
            if (s.element_count)() == 0 {
                element_sort().write(0);
            }
            let id = (s.element_allocate)(*element_sort(), kind, page);
            update_element(s, (s.element)(id), tile, reference, text);
            (s.window_classes)(WC_STORY_BOOK, i32::from(page.0));
            let next = element_sort();
            next.write((*next).wrapping_add(1));
            result.write(id);
        }
        true
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_update_story_page_element(
    s: &Services,
    flags: u16,
    tile: TileIndex,
    id: StoryPageElementID,
    reference: u32,
    text: *const EncodedString,
) -> bool {
    unsafe {
        if !s.deity() {
            return false;
        }
        let element = (s.element)(id);
        if element.is_null() {
            return false;
        }
        let page = *(s.element_page)(element);
        let kind = *(s.element_type)(element);
        if !verify_element(s, page, kind, tile, reference, text) {
            return false;
        }
        if execute(flags) {
            update_element(s, element, tile, reference, text);
            (s.window_classes)(WC_STORY_BOOK, i32::from((*(s.element_page)(element)).0));
        }
        true
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_set_story_page_title(
    s: &Services,
    flags: u16,
    id: StoryPageID,
    text: *const EncodedString,
) -> bool {
    unsafe {
        if !s.deity() {
            return false;
        }
        let page = (s.page)(u32::from(id.0));
        if page.is_null() {
            return false;
        }
        if execute(flags) {
            (s.text_assign)((s.page_title)(page), text);
            (s.window_classes)(WC_STORY_BOOK, i32::from(id.0));
        }
        true
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_set_story_page_date(
    s: &Services,
    flags: u16,
    id: StoryPageID,
    date: i32,
) -> bool {
    unsafe {
        if !s.deity() {
            return false;
        }
        let page = (s.page)(u32::from(id.0));
        if page.is_null() {
            return false;
        }
        if execute(flags) {
            (s.page_date)(page).write(date);
            (s.window_classes)(WC_STORY_BOOK, i32::from(id.0));
        }
        true
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_show_story_page(
    s: &Services,
    flags: u16,
    id: StoryPageID,
) -> bool {
    unsafe {
        if !s.deity() {
            return false;
        }
        let page = (s.page)(u32::from(id.0));
        if page.is_null() {
            return false;
        }
        if execute(flags) {
            let company = *(s.page_company)(page);
            let local = CompanyID((s.company().local_company)());
            if (company != CompanyID::INVALID && company == local)
                || (company == CompanyID::INVALID && s.company_valid(local))
            {
                (s.story_show)(local, id);
            }
        }
        true
    }
}
unsafe fn delete_elements(s: &Services, page: StoryPageID) {
    unsafe {
        each_element(s, |element| {
            if *(s.element_page)(element) == page {
                (s.element_destroy)(element);
            }
        });
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_story_page_destruct(s: &Services, id: StoryPageID) {
    if !(s.page_cleaning)() {
        unsafe {
            delete_elements(s, id);
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_remove_story_page(
    s: &Services,
    flags: u16,
    id: StoryPageID,
) -> bool {
    unsafe {
        if !s.deity() {
            return false;
        }
        let page = (s.page)(u32::from(id.0));
        if page.is_null() {
            return false;
        }
        if execute(flags) {
            delete_elements(s, id);
            (s.company().delete_story_page)(u32::from(id.0));
            (s.window_classes)(WC_STORY_BOOK, -1);
            if (s.page_count)() == 0 {
                s.toolbar_dirty();
            }
        }
        true
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_remove_story_page_element(
    s: &Services,
    flags: u16,
    id: StoryPageElementID,
) -> bool {
    unsafe {
        if !s.deity() {
            return false;
        }
        let element = (s.element)(id);
        if element.is_null() {
            return false;
        }
        if execute(flags) {
            let page = *(s.element_page)(element);
            (s.element_destroy)(element);
            (s.window_classes)(WC_STORY_BOOK, i32::from(page.0));
        }
        true
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_story_page_button(
    s: &Services,
    flags: u16,
    tile: TileIndex,
    id: StoryPageElementID,
    reference: VehicleID,
) -> bool {
    unsafe {
        let element = (s.element)(id);
        if element.is_null() {
            return false;
        }
        let page = *(s.element_page)(element);
        let company = *(s.page_company)((s.page)(u32::from(page.0)));
        let current = CompanyID((s.shared().current_company)());
        if company != CompanyID::INVALID && company != current {
            return false;
        }
        match *(s.element_type)(element) {
            SPET_BUTTON_PUSH => {
                if execute(flags) {
                    (s.push_event)(current, page, id);
                }
            }
            SPET_BUTTON_TILE => {
                if !s.tile_valid(tile) {
                    return false;
                }
                if execute(flags) {
                    (s.tile_event)(current, page, id, tile);
                }
            }
            SPET_BUTTON_VEHICLE => {
                if !(s.vehicle_valid)(reference) {
                    return false;
                }
                if execute(flags) {
                    (s.vehicle_event)(current, page, id, reference);
                }
            }
            _ => return false,
        }
        true
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_valid_link(s: &Services, kind: u8, target: u32) -> bool {
    match kind {
        LT_NONE => target == 0,
        LT_TILE => s.tile_valid(TileIndex(target)),
        LT_INDUSTRY => s.industry_valid(target),
        LT_TOWN => s.town_valid(target),
        LT_COMPANY => u8::try_from(target).is_ok_and(|id| s.company_valid(CompanyID(id))),
        LT_STORY_PAGE => !(s.page)(target).is_null(),
        _ => false,
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_create_league_table(
    s: &Services,
    flags: u16,
    title: *const EncodedString,
    header: *const EncodedString,
    footer: *const EncodedString,
    result: *mut LeagueTableID,
) -> bool {
    unsafe {
        result.write(LeagueTableID::INVALID);
        if !s.deity() || !(s.table_can_allocate)() || (s.text_empty)(title) {
            return false;
        }
        if execute(flags) {
            result.write((s.table_allocate)(title, header, footer));
        }
        true
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_create_league_table_element(
    s: &Services,
    flags: u16,
    table: LeagueTableID,
    rating: i64,
    company: CompanyID,
    text: *const EncodedString,
    score: *const EncodedString,
    kind: u8,
    target: u32,
    result: *mut LeagueTableElementID,
) -> bool {
    unsafe {
        result.write(LeagueTableElementID::INVALID);
        if !s.deity()
            || !(s.entry_can_allocate)()
            || !openttd_rust_valid_link(s, kind, target)
            || (company != CompanyID::INVALID && !s.company_valid(company))
        {
            return false;
        }
        if execute(flags) {
            result.write((s.entry_allocate)(
                table, rating, company, text, score, kind, target,
            ));
            (s.window_data)(WC_COMPANY_LEAGUE, u32::from(table.0), 0);
        }
        true
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_update_league_table_element_data(
    s: &Services,
    flags: u16,
    id: LeagueTableElementID,
    company: CompanyID,
    text: *const EncodedString,
    kind: u8,
    target: u32,
) -> bool {
    unsafe {
        if !s.deity() {
            return false;
        }
        let entry = (s.entry)(id);
        if entry.is_null() {
            return false;
        }
        if (company != CompanyID::INVALID && !s.company_valid(company))
            || !openttd_rust_valid_link(s, kind, target)
        {
            return false;
        }
        if execute(flags) {
            (s.entry_company)(entry).write(company);
            (s.text_assign)((s.entry_text)(entry), text);
            (s.entry_link_type)(entry).write(kind);
            (s.entry_link_target)(entry).write(target);
            (s.window_data)(WC_COMPANY_LEAGUE, u32::from((*(s.entry_table)(entry)).0), 0);
        }
        true
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_update_league_table_element_score(
    s: &Services,
    flags: u16,
    id: LeagueTableElementID,
    rating: i64,
    score: *const EncodedString,
) -> bool {
    unsafe {
        if !s.deity() {
            return false;
        }
        let entry = (s.entry)(id);
        if entry.is_null() {
            return false;
        }
        if execute(flags) {
            (s.entry_rating)(entry).write(rating);
            (s.text_assign)((s.entry_score)(entry), score);
            (s.window_data)(WC_COMPANY_LEAGUE, u32::from((*(s.entry_table)(entry)).0), 0);
        }
        true
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_remove_league_table_element(
    s: &Services,
    flags: u16,
    id: LeagueTableElementID,
) -> bool {
    unsafe {
        if !s.deity() {
            return false;
        }
        let entry = (s.entry)(id);
        if entry.is_null() {
            return false;
        }
        if execute(flags) {
            let table = *(s.entry_table)(entry);
            (s.entry_destroy)(entry);
            (s.window_data)(WC_COMPANY_LEAGUE, u32::from(table.0), 0);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::openttd_rust_story_storage;

    #[test]
    fn story_storage_is_shared_between_serialized_threads() {
        // SAFETY: This test is the sole story-state user; spawn/join serializes each access.
        unsafe {
            let state = openttd_rust_story_storage();
            let page = std::ptr::addr_of_mut!((*state).page_next_sort);
            let element = std::ptr::addr_of_mut!((*state).element_next_sort);
            let old_page = page.read();
            let old_element = element.read();
            page.write(123);
            element.write(456);
            let address = std::thread::spawn(|| {
                let worker = openttd_rust_story_storage();
                assert_eq!(std::ptr::addr_of!((*worker).page_next_sort).read(), 123);
                assert_eq!(std::ptr::addr_of!((*worker).element_next_sort).read(), 456);
                std::ptr::addr_of_mut!((*worker).element_next_sort).write(789);
                worker as usize
            })
            .join()
            .unwrap();
            assert_eq!(address, state as usize);
            assert_eq!(element.read(), 789);
            page.write(old_page);
            element.write(old_element);
        }
    }
}
