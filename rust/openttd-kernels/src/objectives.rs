/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Goals, story pages and league command/lifecycle policy. Canonical pool records and encoded bytes are Rust-owned; native constructors and GUI/VM frames remain.
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
        GT_STORY_PAGE, GT_TILE, GT_TOWN, GoalID, GoalRecord as Goal,
    },
    league_type::{
        LT_COMPANY, LT_INDUSTRY, LT_NONE, LT_STORY_PAGE, LT_TILE, LT_TOWN, LeagueTableElementID,
        LeagueTableElementRecord as LeagueTableElement, LeagueTableID,
    },
    story_type::{
        StoryPageElementID, StoryPageElementRecord as StoryPageElement, StoryPageID,
        StoryPageRecord as StoryPage,
    },
};
#[repr(C)]
pub struct EncodedString {
    _private: [u8; 0],
}
#[repr(C)]
pub struct Services {
    pub shared: *const crate::services::Services,
    pub company: *const crate::company::Services,
    /// Native shell constructors publish their initialized-default records.
    pub goal_allocate: extern "C" fn() -> *mut Goal,
    pub page_allocate: extern "C" fn() -> *mut StoryPage,
    pub element_allocate: extern "C" fn() -> *mut StoryPageElement,
    pub table_allocate: extern "C" fn() -> *mut crate::league_type::LeagueTableRecord,
    pub entry_allocate: extern "C" fn() -> *mut LeagueTableElement,
    pub element_destroy: unsafe extern "C" fn(*mut StoryPageElement),
    pub entry_destroy: unsafe extern "C" fn(*mut LeagueTableElement),
    pub orders: *const crate::orders::Leaves,
    pub subsidy: *const crate::subsidy::Services,
    pub road_commands: *const crate::road_cmd::RoadCommandServices,
    pub vehicle_valid: extern "C" fn(VehicleID) -> bool,
    /// View a borrowed command/GUI input; pool strings are canonical `NativeVec` fields.
    pub text_view: unsafe extern "C" fn(*const EncodedString) -> crate::services::ByteSpan,
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
unsafe fn page_record(id: u32) -> *mut StoryPage {
    if id >= StoryPageID::END as u32 {
        return std::ptr::null_mut();
    }
    unsafe { crate::story_type::story_page_pool().record(StoryPageID(id as u16)) }
}
unsafe fn next_element(from: u32) -> *mut StoryPageElement {
    unsafe {
        let pool = crate::story_type::story_page_element_pool();
        let index = pool.next_index(from as usize, StoryPageElementID::END);
        if index == StoryPageElementID::END {
            std::ptr::null_mut()
        } else {
            pool.record(StoryPageElementID(index as u16))
        }
    }
}
unsafe fn copy_text(
    s: &Services,
    target: *mut crate::industrytype::NativeVec<u8>,
    text: *const EncodedString,
) {
    unsafe {
        crate::owned_bytes::openttd_rust_owned_bytes_assign(target, (s.text_view)(text));
    }
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
    "OpenTTDObjectivesServices" ["probe"] => Services { shared, company, goal_allocate, page_allocate, element_allocate, table_allocate, entry_allocate, element_destroy, entry_destroy, orders, subsidy, road_commands, vehicle_valid, text_view, window_data, window_classes, window_close, question_show, story_show, question_event, push_event, tile_event, vehicle_event };
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
        // Command/lifecycle execution holds the mutex and cannot reset or yield the map.
        let map = crate::map_func::World::new(self.shared());
        crate::tile_map::is_valid_tile(&map.types, tile.0, map.types.count()).unwrap()
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
unsafe fn valid_goal_destination(s: &Services, company: CompanyID, kind: u8, dest: u32) -> bool {
    match kind {
        GT_NONE => dest == 0,
        GT_TILE => s.tile_valid(TileIndex(dest)),
        GT_INDUSTRY => s.industry_valid(dest),
        GT_TOWN => s.town_valid(dest),
        // Company::IsValidID takes the full u32; IDs above u8 lie past the company pool.
        GT_COMPANY => u8::try_from(dest).is_ok_and(|id| s.company_valid(CompanyID(id))),
        GT_STORY_PAGE => {
            let page = unsafe { page_record(dest) };
            if page.is_null() {
                return false;
            }
            let owner = unsafe { (*page).company };
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
pub unsafe extern "C" fn openttd_rust_create_goal_abi(
    s: &Services,
    flags: u16,
    company: CompanyID,
    kind: u8,
    dest: u32,
    text: *const EncodedString,
    result: *mut GoalID,
) -> u8 {
    u8::from(unsafe {
        result.write(GoalID::INVALID);
        if !crate::goal_type::goal_can_allocate()
            || !s.deity()
            || ((s.text_view)(text).size == 0)
            || (company != CompanyID::INVALID && !s.company_valid(company))
            || !valid_goal_destination(s, company, kind, dest)
        {
            return u8::from(false);
        }
        if execute(flags) {
            let goal = (s.goal_allocate)();
            (*goal).r#type = kind;
            (*goal).dst = dest;
            (*goal).company = company;
            copy_text(s, &raw mut (*goal).text, text);
            let id = (*goal).index;
            s.goal_dirty((*goal).company);
            if (*crate::goal_type::goal_descriptor()).items as u32 == 1 {
                s.toolbar_dirty();
            }
            result.write(id);
        }
        true
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_remove_goal_abi(s: &Services, flags: u16, id: GoalID) -> u8 {
    u8::from(unsafe {
        if !s.deity() {
            return u8::from(false);
        }
        let goal = crate::goal_type::goal_pool().record(id);
        if goal.is_null() {
            return u8::from(false);
        }
        if execute(flags) {
            let company = (*goal).company;
            (s.company().delete_goal)(u32::from(id.0));
            s.goal_dirty(company);
            if (*crate::goal_type::goal_descriptor()).items as u32 == 0 {
                s.toolbar_dirty();
            }
        }
        true
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_set_goal_destination_abi(
    s: &Services,
    flags: u16,
    id: GoalID,
    kind: u8,
    dest: u32,
) -> u8 {
    u8::from(unsafe {
        if !s.deity() {
            return u8::from(false);
        }
        let goal = crate::goal_type::goal_pool().record(id);
        if goal.is_null() {
            return u8::from(false);
        }
        if !valid_goal_destination(s, (*goal).company, kind, dest) {
            return u8::from(false);
        }
        if execute(flags) {
            (&raw mut (*goal).r#type).write(kind);
            (&raw mut (*goal).dst).write(dest);
        }
        true
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_set_goal_text_abi(
    s: &Services,
    flags: u16,
    id: GoalID,
    text: *const EncodedString,
) -> u8 {
    u8::from(unsafe {
        if !s.deity() {
            return u8::from(false);
        }
        let goal = crate::goal_type::goal_pool().record(id);
        if goal.is_null() || ((s.text_view)(text).size == 0) {
            return u8::from(false);
        }
        if execute(flags) {
            copy_text(s, (&raw mut (*goal).text), text);
            s.goal_dirty((*goal).company);
        }
        true
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_set_goal_progress_abi(
    s: &Services,
    flags: u16,
    id: GoalID,
    text: *const EncodedString,
) -> u8 {
    u8::from(unsafe {
        if !s.deity() {
            return u8::from(false);
        }
        let goal = crate::goal_type::goal_pool().record(id);
        if goal.is_null() {
            return u8::from(false);
        }
        if execute(flags) {
            copy_text(s, (&raw mut (*goal).progress), text);
            s.goal_dirty((*goal).company);
        }
        true
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_set_goal_completed_abi(
    s: &Services,
    flags: u16,
    id: GoalID,
    completed: bool,
) -> u8 {
    u8::from(unsafe {
        if !s.deity() {
            return u8::from(false);
        }
        let goal = crate::goal_type::goal_pool().record(id);
        if goal.is_null() {
            return u8::from(false);
        }
        if execute(flags) {
            (&raw mut (*goal).completed).write(completed);
            s.goal_dirty((*goal).company);
        }
        true
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_goal_question_abi(
    s: &Services,
    flags: u16,
    id: u16,
    target: u32,
    is_client: bool,
    mut buttons: u32,
    kind: u8,
    text: *const EncodedString,
) -> u8 {
    let company = CompanyID(target as u8);
    buttons &= (1_u32 << GOAL_QUESTION_BUTTON_COUNT) - 1;
    if !s.deity() || unsafe { (s.text_view)(text).size == 0 } {
        return u8::from(false);
    }
    if is_client {
        if !execute(flags)
            && (s.company().network_server)()
            && (s.shared().network_client_info)(target).is_null()
        {
            return u8::from(false);
        }
    } else if company != CompanyID::INVALID && !s.company_valid(company) {
        return u8::from(false);
    }
    let min_buttons = u32::from(kind == GQT_QUESTION);
    if buttons.count_ones() < min_buttons || buttons.count_ones() > 3 || kind >= GQT_END {
        return u8::from(false);
    }
    if execute(flags) {
        if is_client {
            if target != unsafe { ((*s.orders).network_client)() as u32 } {
                return u8::from(true);
            }
        } else {
            let local = CompanyID((s.company().local_company)());
            if company == CompanyID::INVALID && !s.company_valid(local) {
                return u8::from(true);
            }
            if company != CompanyID::INVALID && company != local {
                return u8::from(true);
            }
        }
        unsafe {
            (s.question_show)(id, kind, buttons, text);
        }
    }
    u8::from(true)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_goal_question_answer_abi(
    s: &Services,
    flags: u16,
    id: u16,
    button: u8,
) -> u8 {
    if button >= GOAL_QUESTION_BUTTON_COUNT {
        return u8::from(false);
    }
    if s.deity() {
        if execute(flags) {
            (s.window_close)(WC_GOAL_QUESTION, u32::from(id));
        }
        return u8::from(true);
    }
    if (s.company().finance.networking)()
        && (s.company().local_company)() == (s.shared().current_company)()
    {
        if execute(flags) {
            (s.window_close)(WC_GOAL_QUESTION, u32::from(id));
        }
        if !(s.company().network_server)() {
            return u8::from(true);
        }
    }
    if execute(flags) {
        (s.question_event)(id, CompanyID((s.shared().current_company)()), 1 << button);
    }
    u8::from(true)
}
unsafe fn verify_element(
    s: &Services,
    page: *mut StoryPage,
    kind: u8,
    tile: TileIndex,
    reference: u32,
    text: *const EncodedString,
) -> bool {
    let button = StoryPageButtonData(reference);
    unsafe {
        match kind {
            SPET_TEXT => (s.text_view)(text).size != 0,
            SPET_LOCATION => (s.text_view)(text).size != 0 && s.tile_valid(tile),
            SPET_GOAL => {
                let goal = crate::goal_type::goal_pool().record(GoalID(reference as u16));
                if goal.is_null() {
                    return false;
                }
                !((*page).company == CompanyID::INVALID && (*goal).company != CompanyID::INVALID)
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
        match (*element).r#type {
            SPET_TEXT => copy_text(s, (&raw mut (*element).text), text),
            SPET_LOCATION => {
                copy_text(s, (&raw mut (*element).text), text);
                (&raw mut (*element).referenced_id).write(tile.0);
            }
            SPET_GOAL => (&raw mut (*element).referenced_id).write(reference),
            SPET_BUTTON_PUSH | SPET_BUTTON_TILE | SPET_BUTTON_VEHICLE => {
                copy_text(s, (&raw mut (*element).text), text);
                (&raw mut (*element).referenced_id).write(reference);
            }
            _ => {
                (s.shared().not_reached)();
                std::process::abort();
            }
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_create_story_page_abi(
    s: &Services,
    flags: u16,
    company: CompanyID,
    text: *const EncodedString,
    result: *mut StoryPageID,
) -> u8 {
    u8::from(unsafe {
        result.write(StoryPageID::INVALID);
        if !crate::story_type::story_page_can_allocate()
            || !s.deity()
            || (company != CompanyID::INVALID && !s.company_valid(company))
        {
            return u8::from(false);
        }
        if execute(flags) {
            if (*crate::story_type::story_page_descriptor()).items as u32 == 0 {
                page_sort().write(0);
            }
            let page = (s.page_allocate)();
            (*page).sort_value = *page_sort();
            (*page).date = crate::timer_game_common::calendar_date().0;
            (*page).company = company;
            copy_text(s, &raw mut (*page).title, text);
            let id = (*page).index;
            (s.window_classes)(WC_STORY_BOOK, -1);
            if (*crate::story_type::story_page_descriptor()).items as u32 == 1 {
                s.toolbar_dirty();
            }
            let next = page_sort();
            next.write((*next).wrapping_add(1));
            result.write(id);
        }
        true
    })
}
unsafe fn each_element(mut visit: impl FnMut(*mut StoryPageElement)) {
    let mut from = 0;
    loop {
        let element = unsafe { next_element(from) };
        if element.is_null() {
            break;
        }
        from = u32::from(unsafe { (*element).index }.0) + 1;
        visit(element);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_create_story_page_element_abi(
    s: &Services,
    flags: u16,
    tile: TileIndex,
    page: StoryPageID,
    kind: u8,
    reference: u32,
    text: *const EncodedString,
    result: *mut StoryPageElementID,
) -> u8 {
    u8::from(unsafe {
        result.write(StoryPageElementID::INVALID);
        if !crate::story_type::story_page_element_can_allocate() {
            return u8::from(false);
        }
        let mut count = 0_u16;
        each_element(|element| {
            if (*element).page == page {
                count = count.wrapping_add(1);
            }
        });
        if count >= 128 || !s.deity() {
            return u8::from(false);
        }
        let page_record = page_record(u32::from(page.0));
        if page_record.is_null() || !verify_element(s, page_record, kind, tile, reference, text) {
            return u8::from(false);
        }
        if execute(flags) {
            if (*crate::story_type::story_page_element_descriptor()).items as u32 == 0 {
                element_sort().write(0);
            }
            let element = (s.element_allocate)();
            (*element).sort_value = *element_sort();
            (*element).r#type = kind;
            (*element).page = page;
            let id = (*element).index;
            update_element(s, element, tile, reference, text);
            (s.window_classes)(WC_STORY_BOOK, i32::from(page.0));
            let next = element_sort();
            next.write((*next).wrapping_add(1));
            result.write(id);
        }
        true
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_update_story_page_element_abi(
    s: &Services,
    flags: u16,
    tile: TileIndex,
    id: StoryPageElementID,
    reference: u32,
    text: *const EncodedString,
) -> u8 {
    u8::from(unsafe {
        if !s.deity() {
            return u8::from(false);
        }
        let element = crate::story_type::story_page_element_pool().record(id);
        if element.is_null() {
            return u8::from(false);
        }
        let page = (*element).page;
        let kind = (*element).r#type;
        let page_record = if kind == SPET_GOAL {
            page_record(u32::from(page.0))
        } else {
            std::ptr::null_mut()
        };
        if !verify_element(s, page_record, kind, tile, reference, text) {
            return u8::from(false);
        }
        if execute(flags) {
            update_element(s, element, tile, reference, text);
            (s.window_classes)(WC_STORY_BOOK, i32::from(((*element).page).0));
        }
        true
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_set_story_page_title_abi(
    s: &Services,
    flags: u16,
    id: StoryPageID,
    text: *const EncodedString,
) -> u8 {
    u8::from(unsafe {
        if !s.deity() {
            return u8::from(false);
        }
        let page = page_record(u32::from(id.0));
        if page.is_null() {
            return u8::from(false);
        }
        if execute(flags) {
            copy_text(s, (&raw mut (*page).title), text);
            (s.window_classes)(WC_STORY_BOOK, i32::from(id.0));
        }
        true
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_set_story_page_date_abi(
    s: &Services,
    flags: u16,
    id: StoryPageID,
    date: i32,
) -> u8 {
    u8::from(unsafe {
        if !s.deity() {
            return u8::from(false);
        }
        let page = page_record(u32::from(id.0));
        if page.is_null() {
            return u8::from(false);
        }
        if execute(flags) {
            (&raw mut (*page).date).write(date);
            (s.window_classes)(WC_STORY_BOOK, i32::from(id.0));
        }
        true
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_show_story_page_abi(
    s: &Services,
    flags: u16,
    id: StoryPageID,
) -> u8 {
    u8::from(unsafe {
        if !s.deity() {
            return u8::from(false);
        }
        let page = page_record(u32::from(id.0));
        if page.is_null() {
            return u8::from(false);
        }
        if execute(flags) {
            let company = (*page).company;
            let local = CompanyID((s.company().local_company)());
            if (company != CompanyID::INVALID && company == local)
                || (company == CompanyID::INVALID && s.company_valid(local))
            {
                (s.story_show)(local, id);
            }
        }
        true
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_story_page_destroy(s: &Services, page: StoryPageID) {
    unsafe {
        if (*crate::story_type::story_page_descriptor()).cleaning {
            return;
        }
        each_element(|element| {
            if (*element).page == page {
                (s.element_destroy)(element);
            }
        });
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_remove_story_page_abi(
    s: &Services,
    flags: u16,
    id: StoryPageID,
) -> u8 {
    u8::from(unsafe {
        if !s.deity() {
            return u8::from(false);
        }
        let page = page_record(u32::from(id.0));
        if page.is_null() {
            return u8::from(false);
        }
        if execute(flags) {
            (s.company().delete_story_page)(u32::from(id.0));
            (s.window_classes)(WC_STORY_BOOK, -1);
            if (*crate::story_type::story_page_descriptor()).items as u32 == 0 {
                s.toolbar_dirty();
            }
        }
        true
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_remove_story_page_element_abi(
    s: &Services,
    flags: u16,
    id: StoryPageElementID,
) -> u8 {
    u8::from(unsafe {
        if !s.deity() {
            return u8::from(false);
        }
        let element = crate::story_type::story_page_element_pool().record(id);
        if element.is_null() {
            return u8::from(false);
        }
        if execute(flags) {
            let page = (*element).page;
            (s.element_destroy)(element);
            (s.window_classes)(WC_STORY_BOOK, i32::from(page.0));
        }
        true
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_story_page_button_abi(
    s: &Services,
    flags: u16,
    tile: TileIndex,
    id: StoryPageElementID,
    reference: VehicleID,
) -> u8 {
    u8::from(unsafe {
        let element = crate::story_type::story_page_element_pool().record(id);
        if element.is_null() {
            return u8::from(false);
        }
        let page = (*element).page;
        let company = (*(page_record(u32::from(page.0)))).company;
        let current = CompanyID((s.shared().current_company)());
        if company != CompanyID::INVALID && company != current {
            return u8::from(false);
        }
        match (*element).r#type {
            SPET_BUTTON_PUSH => {
                if execute(flags) {
                    (s.push_event)(current, page, id);
                }
            }
            SPET_BUTTON_TILE => {
                if !s.tile_valid(tile) {
                    return u8::from(false);
                }
                if execute(flags) {
                    (s.tile_event)(current, page, id, tile);
                }
            }
            SPET_BUTTON_VEHICLE => {
                if !(s.vehicle_valid)(reference) {
                    return u8::from(false);
                }
                if execute(flags) {
                    (s.vehicle_event)(current, page, id, reference);
                }
            }
            _ => return u8::from(false),
        }
        true
    })
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_valid_link_abi(s: &Services, kind: u8, target: u32) -> u8 {
    u8::from(match kind {
        LT_NONE => target == 0,
        LT_TILE => s.tile_valid(TileIndex(target)),
        LT_INDUSTRY => s.industry_valid(target),
        LT_TOWN => s.town_valid(target),
        LT_COMPANY => u8::try_from(target).is_ok_and(|id| s.company_valid(CompanyID(id))),
        LT_STORY_PAGE => !unsafe { page_record(target) }.is_null(),
        _ => false,
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_create_league_table_abi(
    s: &Services,
    flags: u16,
    title: *const EncodedString,
    header: *const EncodedString,
    footer: *const EncodedString,
    result: *mut LeagueTableID,
) -> u8 {
    u8::from(unsafe {
        result.write(LeagueTableID::INVALID);
        if !s.deity()
            || !crate::league_type::league_table_can_allocate()
            || ((s.text_view)(title).size == 0)
        {
            return u8::from(false);
        }
        if execute(flags) {
            let table = (s.table_allocate)();
            copy_text(s, &raw mut (*table).title, title);
            copy_text(s, &raw mut (*table).header, header);
            copy_text(s, &raw mut (*table).footer, footer);
            result.write((*table).index);
        }
        true
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_create_league_table_element_abi(
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
) -> u8 {
    u8::from(unsafe {
        result.write(LeagueTableElementID::INVALID);
        if !s.deity()
            || !crate::league_type::league_table_element_can_allocate()
            || openttd_rust_valid_link_abi(s, kind, target) == 0
            || (company != CompanyID::INVALID && !s.company_valid(company))
        {
            return u8::from(false);
        }
        if execute(flags) {
            let entry = (s.entry_allocate)();
            (*entry).table = table;
            (*entry).rating = rating;
            (*entry).company = company;
            copy_text(s, &raw mut (*entry).text, text);
            copy_text(s, &raw mut (*entry).score, score);
            (*entry).link_type = kind;
            (*entry).link_target = target;
            result.write((*entry).index);
            (s.window_data)(WC_COMPANY_LEAGUE, u32::from(table.0), 0);
        }
        true
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_update_league_table_element_data_abi(
    s: &Services,
    flags: u16,
    id: LeagueTableElementID,
    company: CompanyID,
    text: *const EncodedString,
    kind: u8,
    target: u32,
) -> u8 {
    u8::from(unsafe {
        if !s.deity() {
            return u8::from(false);
        }
        let entry = crate::league_type::league_table_element_pool().record(id);
        if entry.is_null() {
            return u8::from(false);
        }
        if (company != CompanyID::INVALID && !s.company_valid(company))
            || openttd_rust_valid_link_abi(s, kind, target) == 0
        {
            return u8::from(false);
        }
        if execute(flags) {
            (&raw mut (*entry).company).write(company);
            copy_text(s, (&raw mut (*entry).text), text);
            (&raw mut (*entry).link_type).write(kind);
            (&raw mut (*entry).link_target).write(target);
            (s.window_data)(WC_COMPANY_LEAGUE, u32::from(((*entry).table).0), 0);
        }
        true
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_update_league_table_element_score_abi(
    s: &Services,
    flags: u16,
    id: LeagueTableElementID,
    rating: i64,
    score: *const EncodedString,
) -> u8 {
    u8::from(unsafe {
        if !s.deity() {
            return u8::from(false);
        }
        let entry = crate::league_type::league_table_element_pool().record(id);
        if entry.is_null() {
            return u8::from(false);
        }
        if execute(flags) {
            (&raw mut (*entry).rating).write(rating);
            copy_text(s, (&raw mut (*entry).score), score);
            (s.window_data)(WC_COMPANY_LEAGUE, u32::from(((*entry).table).0), 0);
        }
        true
    })
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_remove_league_table_element_abi(
    s: &Services,
    flags: u16,
    id: LeagueTableElementID,
) -> u8 {
    u8::from(unsafe {
        if !s.deity() {
            return u8::from(false);
        }
        let entry = crate::league_type::league_table_element_pool().record(id);
        if entry.is_null() {
            return u8::from(false);
        }
        if execute(flags) {
            let table = (*entry).table;
            (s.entry_destroy)(entry);
            (s.window_data)(WC_COMPANY_LEAGUE, u32::from(table.0), 0);
        }
        true
    })
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
