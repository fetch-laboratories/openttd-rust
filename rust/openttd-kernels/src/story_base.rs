/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Packed button data from `story_base.h`; native facades preserve setter assertions.
#![allow(missing_docs)]
pub const SPET_TEXT: u8 = 0;
pub const SPET_LOCATION: u8 = 1;
pub const SPET_GOAL: u8 = 2;
pub const SPET_BUTTON_PUSH: u8 = 3;
pub const SPET_BUTTON_TILE: u8 = 4;
pub const SPET_BUTTON_VEHICLE: u8 = 5;
pub const SPBF_FLOAT_LEFT: u8 = 1;
pub const SPBF_FLOAT_RIGHT: u8 = 2;
pub const SPBC_END: u8 = 55;
pub struct StoryPageButtonData(pub u32);
impl StoryPageButtonData {
    pub fn validate_colour(&self) -> bool {
        (self.0 & 0xFF) < u32::from(crate::gfx_type::COLOUR_END)
    }
    pub fn validate_flags(&self) -> bool {
        let flags = (self.0 >> 24) as u8;
        (flags & SPBF_FLOAT_LEFT == 0 || flags & SPBF_FLOAT_RIGHT == 0)
            && flags & !(SPBF_FLOAT_LEFT | SPBF_FLOAT_RIGHT) == 0
    }
    pub fn validate_cursor(&self) -> bool {
        ((self.0 >> 8) & 0xFF) < u32::from(SPBC_END)
    }
    pub fn validate_vehicle_type(&self) -> bool {
        let kind = ((self.0 >> 16) & 0xFF) as u8;
        kind == u8::MAX || kind < crate::vehicle_type::VEH_COMPANY_END
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_story_button_validate_colour(data: u32) -> bool {
    StoryPageButtonData(data).validate_colour()
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_story_button_validate_flags(data: u32) -> bool {
    StoryPageButtonData(data).validate_flags()
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_story_button_validate_cursor(data: u32) -> bool {
    StoryPageButtonData(data).validate_cursor()
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_story_button_validate_vehicle_type(data: u32) -> bool {
    StoryPageButtonData(data).validate_vehicle_type()
}
