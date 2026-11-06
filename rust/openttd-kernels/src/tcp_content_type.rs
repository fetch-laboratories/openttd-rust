/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `ContentType`, `PacketContentType` and the `ContentInfo` scalars from
//! `network/core/tcp_content_type.h`. The scalars are the base record of the native
//! `ContentInfo`, whose text and list fields stay native; both languages access them in place.
#![allow(unsafe_code)]

pub const CONTENT_TYPE_BEGIN: u8 = 1;
pub const CONTENT_TYPE_BASE_GRAPHICS: u8 = 1;
pub const CONTENT_TYPE_NEWGRF: u8 = 2;
pub const CONTENT_TYPE_AI: u8 = 3;
pub const CONTENT_TYPE_AI_LIBRARY: u8 = 4;
pub const CONTENT_TYPE_SCENARIO: u8 = 5;
pub const CONTENT_TYPE_HEIGHTMAP: u8 = 6;
pub const CONTENT_TYPE_BASE_SOUNDS: u8 = 7;
pub const CONTENT_TYPE_BASE_MUSIC: u8 = 8;
pub const CONTENT_TYPE_GAME: u8 = 9;
pub const CONTENT_TYPE_GAME_LIBRARY: u8 = 10;
pub const CONTENT_TYPE_END: u8 = 11;

pub const PACKET_CONTENT_CLIENT_INFO_LIST: u8 = 0;
pub const PACKET_CONTENT_CLIENT_INFO_ID: u8 = 1;
pub const PACKET_CONTENT_CLIENT_INFO_EXTID: u8 = 2;
pub const PACKET_CONTENT_CLIENT_INFO_EXTID_MD5: u8 = 3;
pub const PACKET_CONTENT_SERVER_INFO: u8 = 4;
pub const PACKET_CONTENT_CLIENT_CONTENT: u8 = 5;
pub const PACKET_CONTENT_SERVER_CONTENT: u8 = 6;
pub const PACKET_CONTENT_END: u8 = 7;

/// `ContentInfo::State`.
pub mod state {
    pub const UNSELECTED: u8 = 0;
    pub const SELECTED: u8 = 1;
    pub const AUTOSELECTED: u8 = 2;
    pub const ALREADY_HERE: u8 = 3;
    pub const DOES_NOT_EXIST: u8 = 4;
    pub const INVALID: u8 = 5;
}

crate::abi::shared_records! {
    fn abi_record;
    /// Base record of the native `ContentInfo`; native text and list fields follow it.
    "OpenTTDContentInfo" ["content"] => pub struct ContentInfo {
        pub r#type: u8 => "ContentType",
        pub id: u32 => "ContentID",
        pub filesize: u32,
        pub unique_id: u32,
        pub md5sum: [u8; 16] => "MD5Hash",
        pub state: u8 => "ContentInfoState",
        pub upgrade: bool,
    }
}

impl ContentInfo {
    /// `ContentInfo::IsSelected`.
    pub fn is_selected(&self) -> bool {
        matches!(
            self.state,
            state::SELECTED | state::AUTOSELECTED | state::ALREADY_HERE
        )
    }
    /// `ContentInfo::IsValid`.
    pub fn is_valid(&self) -> bool {
        self.state < state::INVALID
            && self.r#type >= CONTENT_TYPE_BEGIN
            && self.r#type < CONTENT_TYPE_END
    }
}

/// `ContentInfo::IsSelected` for the native method.
/// # Safety
/// `info` is a live record.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_content_is_selected(info: *const ContentInfo) -> u8 {
    u8::from(unsafe { &*info }.is_selected())
}

/// `ContentInfo::IsValid` for the native method.
/// # Safety
/// `info` is a live record.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_content_is_valid(info: *const ContentInfo) -> u8 {
    u8::from(unsafe { &*info }.is_valid())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_and_validity_follow_the_original_state_order() {
        let mut info = ContentInfo {
            r#type: CONTENT_TYPE_AI,
            id: 1,
            filesize: 0,
            unique_id: 0,
            md5sum: [0; 16],
            state: 0,
            upgrade: false,
        };
        let selected = [false, true, true, true, false, false];
        for (value, expected) in selected.into_iter().enumerate() {
            info.state = u8::try_from(value).unwrap();
            assert_eq!(info.is_selected(), expected);
            assert_eq!(info.is_valid(), value < 5);
        }
        info.state = state::UNSELECTED;
        for (kind, expected) in [(0, false), (1, true), (10, true), (11, false), (255, false)] {
            info.r#type = kind;
            assert_eq!(info.is_valid(), expected);
        }
    }
}
