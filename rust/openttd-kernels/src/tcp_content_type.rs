/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `ContentType`, `PacketContentType` and the `ContentInfo` scalars from
//! `network/core/tcp_content_type.h`. The scalars are the base record of the native
//! `ContentInfo`, whose text and list fields stay native; both languages access them in place.
#![allow(unsafe_code)]

pub const CONTENT_TYPE_BEGIN: u8 = ContentType::CONTENT_TYPE_BEGIN.0;
pub const CONTENT_TYPE_BASE_GRAPHICS: u8 = ContentType::CONTENT_TYPE_BASE_GRAPHICS.0;
pub const CONTENT_TYPE_NEWGRF: u8 = ContentType::CONTENT_TYPE_NEWGRF.0;
pub const CONTENT_TYPE_AI: u8 = ContentType::CONTENT_TYPE_AI.0;
pub const CONTENT_TYPE_AI_LIBRARY: u8 = ContentType::CONTENT_TYPE_AI_LIBRARY.0;
pub const CONTENT_TYPE_SCENARIO: u8 = ContentType::CONTENT_TYPE_SCENARIO.0;
pub const CONTENT_TYPE_HEIGHTMAP: u8 = ContentType::CONTENT_TYPE_HEIGHTMAP.0;
pub const CONTENT_TYPE_BASE_SOUNDS: u8 = ContentType::CONTENT_TYPE_BASE_SOUNDS.0;
pub const CONTENT_TYPE_BASE_MUSIC: u8 = ContentType::CONTENT_TYPE_BASE_MUSIC.0;
pub const CONTENT_TYPE_GAME: u8 = ContentType::CONTENT_TYPE_GAME.0;
pub const CONTENT_TYPE_GAME_LIBRARY: u8 = ContentType::CONTENT_TYPE_GAME_LIBRARY.0;
pub const CONTENT_TYPE_END: u8 = ContentType::CONTENT_TYPE_END.0;

pub const PACKET_CONTENT_CLIENT_INFO_LIST: u8 =
    PacketContentType::PACKET_CONTENT_CLIENT_INFO_LIST.0;
pub const PACKET_CONTENT_CLIENT_INFO_ID: u8 = PacketContentType::PACKET_CONTENT_CLIENT_INFO_ID.0;
pub const PACKET_CONTENT_CLIENT_INFO_EXTID: u8 =
    PacketContentType::PACKET_CONTENT_CLIENT_INFO_EXTID.0;
pub const PACKET_CONTENT_CLIENT_INFO_EXTID_MD5: u8 =
    PacketContentType::PACKET_CONTENT_CLIENT_INFO_EXTID_MD5.0;
pub const PACKET_CONTENT_SERVER_INFO: u8 = PacketContentType::PACKET_CONTENT_SERVER_INFO.0;
pub const PACKET_CONTENT_CLIENT_CONTENT: u8 = PacketContentType::PACKET_CONTENT_CLIENT_CONTENT.0;
pub const PACKET_CONTENT_SERVER_CONTENT: u8 = PacketContentType::PACKET_CONTENT_SERVER_CONTENT.0;
pub const PACKET_CONTENT_END: u8 = PacketContentType::PACKET_CONTENT_END.0;

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

/// Native `ContentType`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentType(pub u8);

impl ContentType {
    /// Helper to mark the begin of the types
    pub const CONTENT_TYPE_BEGIN: Self = Self(1);
    /// The content consists of base graphics
    pub const CONTENT_TYPE_BASE_GRAPHICS: Self = Self(1);
    /// The content consists of a `NewGRF`
    pub const CONTENT_TYPE_NEWGRF: Self = Self(2);
    /// The content consists of an AI
    pub const CONTENT_TYPE_AI: Self = Self(3);
    /// The content consists of an AI library
    pub const CONTENT_TYPE_AI_LIBRARY: Self = Self(4);
    /// The content consists of a scenario
    pub const CONTENT_TYPE_SCENARIO: Self = Self(5);
    /// The content consists of a heightmap
    pub const CONTENT_TYPE_HEIGHTMAP: Self = Self(6);
    /// The content consists of base sounds
    pub const CONTENT_TYPE_BASE_SOUNDS: Self = Self(7);
    /// The content consists of base music
    pub const CONTENT_TYPE_BASE_MUSIC: Self = Self(8);
    /// The content consists of a game script
    pub const CONTENT_TYPE_GAME: Self = Self(9);
    /// The content consists of a GS library
    pub const CONTENT_TYPE_GAME_LIBRARY: Self = Self(10);
    /// Helper to mark the end of the types
    pub const CONTENT_TYPE_END: Self = Self(11);
    /// Invalid/uninitialized content
    pub const INVALID_CONTENT_TYPE: Self = Self(255);
}

/// Native `PacketContentType`, retaining every underlying `u8` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PacketContentType(pub u8);

impl PacketContentType {
    /// Queries the content server for a list of info of a given content type
    pub const PACKET_CONTENT_CLIENT_INFO_LIST: Self = Self(0);
    /// Queries the content server for information about a list of internal IDs
    pub const PACKET_CONTENT_CLIENT_INFO_ID: Self = Self(1);
    /// Queries the content server for information about a list of external IDs
    pub const PACKET_CONTENT_CLIENT_INFO_EXTID: Self = Self(2);
    /// Queries the content server for information about a list of external IDs and MD5
    pub const PACKET_CONTENT_CLIENT_INFO_EXTID_MD5: Self = Self(3);
    /// Reply of content server with information about content
    pub const PACKET_CONTENT_SERVER_INFO: Self = Self(4);
    /// Request a content file given an internal ID
    pub const PACKET_CONTENT_CLIENT_CONTENT: Self = Self(5);
    /// Reply with the content of the given ID
    pub const PACKET_CONTENT_SERVER_CONTENT: Self = Self(6);
    /// Must ALWAYS be on the end of this list!! (period)
    pub const PACKET_CONTENT_END: Self = Self(7);
}

/// Native `ContentID`, retaining every underlying `u32` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ContentID(pub u32);

impl ContentID {
    /// Invalid content identifier.
    pub const INVALID_CONTENT_ID: Self = Self(u32::MAX);
}
