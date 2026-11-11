/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `ContentType`, `PacketContentType` and the `ContentInfo` scalars from
//! `network/core/tcp_content_type.h`. The scalars are the base record of the native
//! `ContentInfo`, whose text and list fields stay native; both languages access them in place.
#![allow(unsafe_code)]

/// `ContentInfo::State`, retaining every underlying byte.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContentInfoState(pub u8);

#[allow(
    non_upper_case_globals,
    reason = "original ContentInfo::State enumerator vocabulary"
)]
impl ContentInfoState {
    /// The content has not been selected.
    pub const Unselected: Self = Self(0);
    /// The content has been manually selected.
    pub const Selected: Self = Self(1);
    /// The content has been selected as a dependency.
    pub const Autoselected: Self = Self(2);
    /// The content is already at the client side.
    pub const AlreadyHere: Self = Self(3);
    /// The content does not exist in the content system.
    pub const DoesNotExist: Self = Self(4);
    /// The content is invalid.
    pub const Invalid: Self = Self(5);
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
    /// Typed view of the shared native content type.
    pub fn content_type(&self) -> ContentType {
        ContentType(self.r#type)
    }
    /// Typed view of the shared native content identifier.
    pub fn id(&self) -> ContentID {
        ContentID(self.id)
    }
    /// Typed view of the shared native selection state.
    pub fn state(&self) -> ContentInfoState {
        ContentInfoState(self.state)
    }
    /// Store the selection state in its shared native field.
    pub fn set_state(&mut self, state: ContentInfoState) {
        self.state = state.0;
    }

    /// `ContentInfo::IsSelected`.
    pub fn is_selected(&self) -> bool {
        matches!(
            self.state(),
            ContentInfoState::Selected
                | ContentInfoState::Autoselected
                | ContentInfoState::AlreadyHere
        )
    }
    /// `ContentInfo::IsValid`.
    pub fn is_valid(&self) -> bool {
        self.state() < ContentInfoState::Invalid
            && self.content_type() >= ContentType::CONTENT_TYPE_BEGIN
            && self.content_type() < ContentType::CONTENT_TYPE_END
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
            r#type: ContentType::CONTENT_TYPE_AI.0,
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
        info.state = ContentInfoState::Unselected.0;
        for (kind, expected) in [(0, false), (1, true), (10, true), (11, false), (255, false)] {
            info.r#type = kind;
            assert_eq!(info.is_valid(), expected);
        }
    }
}

/// Native `ContentType`: these values are database keys. Every underlying byte is retained.
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

/// Native `PacketContentType`: packet order must not change. Every underlying byte is retained.
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
