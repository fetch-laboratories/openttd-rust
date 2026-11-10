/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared `table/control_codes.h` constants used by GRF text translation.

/// Native `StringControlCode`, retaining every underlying `uint16_t` value.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct StringControlCode(pub u16);

// All SCC_ENCODED* control codes must have stable ids as they are stored in
// strings that are saved in savegames.
impl StringControlCode {
    /// Switch to large font
    pub const SCC_BIGFONT: Self = Self(0xe006);
    /// `SCC_BLACK` from `table/control_codes.h`.
    pub const SCC_BLACK: Self = Self(0xe052);
    /// `SCC_BLUE` from `table/control_codes.h`.
    pub const SCC_BLUE: Self = Self(0xe042);
    /// `SCC_BROWN` from `table/control_codes.h`.
    pub const SCC_BROWN: Self = Self(0xe04d);
    /// `SCC_BUS` from `table/control_codes.h`.
    pub const SCC_BUS: Self = Self(0xe2b6);
    /// `SCC_CHECKMARK` from `table/control_codes.h`.
    pub const SCC_CHECKMARK: Self = Self(0xe2ac);
    /// `SCC_CITY` from `table/control_codes.h`.
    pub const SCC_CITY: Self = Self(0xe29c);
    /// `SCC_CREAM` from `table/control_codes.h`.
    pub const SCC_CREAM: Self = Self(0xe04c);
    /// `SCC_CROSS` from `table/control_codes.h`.
    pub const SCC_CROSS: Self = Self(0xe2ad);
    /// `SCC_DKBLUE` from `table/control_codes.h`.
    pub const SCC_DKBLUE: Self = Self(0xe051);
    /// `SCC_DKGREEN` from `table/control_codes.h`.
    pub const SCC_DKGREEN: Self = Self(0xe04b);
    /// `SCC_DOWN_ARROW` from `table/control_codes.h`.
    pub const SCC_DOWN_ARROW: Self = Self(0xe2aa);
    /// `SCC_GENDER_INDEX` from `table/control_codes.h`.
    pub const SCC_GENDER_INDEX: Self = Self(0xe03e);
    /// `SCC_GENDER_LIST` from `table/control_codes.h`.
    pub const SCC_GENDER_LIST: Self = Self(0xe03d);
    /// `SCC_GOLD` from `table/control_codes.h`.
    pub const SCC_GOLD: Self = Self(0xe044);
    /// `SCC_GRAY` from `table/control_codes.h`.
    pub const SCC_GRAY: Self = Self(0xe050);
    /// `SCC_GREEN` from `table/control_codes.h`.
    pub const SCC_GREEN: Self = Self(0xe049);
    /// `SCC_LORRY` from `table/control_codes.h`.
    pub const SCC_LORRY: Self = Self(0xe2b5);
    /// `SCC_LTBLUE` from `table/control_codes.h`.
    pub const SCC_LTBLUE: Self = Self(0xe04f);
    /// `SCC_LTBROWN` from `table/control_codes.h`.
    pub const SCC_LTBROWN: Self = Self(0xe047);
    /// 85: Discard the next two bytes
    pub const SCC_NEWGRF_DISCARD_WORD: Self = Self(0xe072);
    /// 9A 06: Read 1 byte from the stack and print it as hex
    pub const SCC_NEWGRF_PRINT_BYTE_HEX: Self = Self(0xe063);
    /// 9A 16: Read 4 bytes from the stack as base 0 date
    pub const SCC_NEWGRF_PRINT_DWORD_DATE_LONG: Self = Self(0xe067);
    /// 9A 21: Read 4 bytes from the stack as unsigned force
    pub const SCC_NEWGRF_PRINT_DWORD_FORCE: Self = Self(0xe070);
    /// 9A 08: Read 4 bytes from the stack and print it as hex
    pub const SCC_NEWGRF_PRINT_DWORD_HEX: Self = Self(0xe065);
    /// 7B: Read 4 bytes from the stack
    pub const SCC_NEWGRF_PRINT_DWORD_SIGNED: Self = Self(0xe056);
    /// 9A 01: Read 8 bytes from the stack as currency
    pub const SCC_NEWGRF_PRINT_QWORD_CURRENCY: Self = Self(0xe062);
    /// 9A 0B: Read 8 bytes from the stack and print it as hex
    pub const SCC_NEWGRF_PRINT_QWORD_HEX: Self = Self(0xe066);
    /// 82: Read 2 bytes from the stack as base 1920 date
    pub const SCC_NEWGRF_PRINT_WORD_DATE_LONG: Self = Self(0xe05c);
    /// 9A 07: Read 2 bytes from the stack and print it as hex
    pub const SCC_NEWGRF_PRINT_WORD_HEX: Self = Self(0xe064);
    /// 9A 0C: Read 2 bytes from the stack as station name
    pub const SCC_NEWGRF_PRINT_WORD_STATION_NAME: Self = Self(0xe061);
    /// 81: Read 2 bytes from the stack as String ID
    pub const SCC_NEWGRF_PRINT_WORD_STRING_ID: Self = Self(0xe05b);
    /// 87: Read 2 bytes from the stack as long signed volume
    pub const SCC_NEWGRF_PRINT_WORD_VOLUME_LONG: Self = Self(0xe05f);
    /// 9A 0D: Read 2 bytes from the stack as long unsigned weight
    pub const SCC_NEWGRF_PRINT_WORD_WEIGHT_LONG: Self = Self(0xe060);
    /// 9A 03: Pushes 2 bytes onto the stack
    pub const SCC_NEWGRF_PUSH_WORD: Self = Self(0xe071);
    /// 86: Rotate the top 4 words of the stack (W4 W1 W2 W3)
    pub const SCC_NEWGRF_ROTATE_TOP_4_WORDS: Self = Self(0xe073);
    /// Inline another string at the current position, `StringID` is encoded in the string
    pub const SCC_NEWGRF_STRINL: Self = Self(0xe074);
    /// `SCC_ORANGE` from `table/control_codes.h`.
    pub const SCC_ORANGE: Self = Self(0xe048);
    /// `SCC_PLANE` from `table/control_codes.h`.
    pub const SCC_PLANE: Self = Self(0xe2b7);
    /// `SCC_PLURAL_LIST` from `table/control_codes.h`.
    pub const SCC_PLURAL_LIST: Self = Self(0xe03c);
    /// `SCC_POP_COLOUR` from `table/control_codes.h`.
    pub const SCC_POP_COLOUR: Self = Self(0xe055);
    /// `SCC_PURPLE` from `table/control_codes.h`.
    pub const SCC_PURPLE: Self = Self(0xe046);
    /// `SCC_PUSH_COLOUR` from `table/control_codes.h`.
    pub const SCC_PUSH_COLOUR: Self = Self(0xe054);
    /// `SCC_RED` from `table/control_codes.h`.
    pub const SCC_RED: Self = Self(0xe045);
    /// `SCC_RIGHT_ARROW` from `table/control_codes.h`.
    pub const SCC_RIGHT_ARROW: Self = Self(0xe2af);
    /// `SCC_SET_CASE` from `table/control_codes.h`.
    pub const SCC_SET_CASE: Self = Self(0xe040);
    /// `SCC_SHIP` from `table/control_codes.h`.
    pub const SCC_SHIP: Self = Self(0xe2b8);
    /// `SCC_SILVER` from `table/control_codes.h`.
    pub const SCC_SILVER: Self = Self(0xe043);
    /// `SCC_SMALL_DOWN_ARROW` from `table/control_codes.h`.
    pub const SCC_SMALL_DOWN_ARROW: Self = Self(0xe2bd);
    /// `SCC_SMALL_UP_ARROW` from `table/control_codes.h`.
    pub const SCC_SMALL_UP_ARROW: Self = Self(0xe2bc);
    /// `SCC_SUPERSCRIPT_M1` from `table/control_codes.h`.
    pub const SCC_SUPERSCRIPT_M1: Self = Self(0xe2b9);
    /// `SCC_SWITCH_CASE` from `table/control_codes.h`.
    pub const SCC_SWITCH_CASE: Self = Self(0xe041);
    /// Switch to small font
    pub const SCC_TINYFONT: Self = Self(0xe005);
    /// `SCC_TOWN` from `table/control_codes.h`.
    pub const SCC_TOWN: Self = Self(0xe29b);
    /// `SCC_TRAIN` from `table/control_codes.h`.
    pub const SCC_TRAIN: Self = Self(0xe2b4);
    /// `SCC_UP_ARROW` from `table/control_codes.h`.
    pub const SCC_UP_ARROW: Self = Self(0xe2a0);
    /// `SCC_WHITE` from `table/control_codes.h`.
    pub const SCC_WHITE: Self = Self(0xe04e);
    /// `SCC_YELLOW` from `table/control_codes.h`.
    pub const SCC_YELLOW: Self = Self(0xe04a);
    /// Record separator for encoded strings.
    pub const SCC_RECORD_SEPARATOR: Self = Self(0x1e);
    /// First nonprintable string control code.
    pub const SCC_CONTROL_START: Self = Self(0xe000);
    /// Last nonprintable string control code.
    pub const SCC_CONTROL_END: Self = Self(0xe1ff);
    /// First special printable sprite symbol; these map to original glyphs.
    pub const SCC_SPRITE_START: Self = Self(0xe200);
    /// Last special printable sprite symbol.
    pub const SCC_SPRITE_END: Self = Self(Self::SCC_SPRITE_START.0 + 0xff);
    /// Encoded string marker and sub-string parameter.
    pub const SCC_ENCODED: Self = Self(Self::SCC_CONTROL_START.0);
    /// Encoded text from `OpenTTD`.
    pub const SCC_ENCODED_INTERNAL: Self = Self(Self::SCC_ENCODED.0 + 1);
    /// Encoded numeric parameter.
    pub const SCC_ENCODED_NUMERIC: Self = Self(Self::SCC_ENCODED.0 + 2);
    /// Encoded string parameter.
    pub const SCC_ENCODED_STRING: Self = Self(Self::SCC_ENCODED.0 + 3);
    /// Promote the native enum to the original `char32_t` codepoint domain.
    #[must_use]
    pub const fn codepoint(self) -> u32 {
        self.0 as u32
    }
}

/// `SCC_BIGFONT` in the native codepoint domain.
pub const SCC_BIGFONT: u32 = StringControlCode::SCC_BIGFONT.codepoint();
/// `SCC_BLACK` in the native codepoint domain.
pub const SCC_BLACK: u32 = StringControlCode::SCC_BLACK.codepoint();
/// `SCC_BLUE` in the native codepoint domain.
pub const SCC_BLUE: u32 = StringControlCode::SCC_BLUE.codepoint();
/// `SCC_BROWN` in the native codepoint domain.
pub const SCC_BROWN: u32 = StringControlCode::SCC_BROWN.codepoint();
/// `SCC_BUS` in the native codepoint domain.
pub const SCC_BUS: u32 = StringControlCode::SCC_BUS.codepoint();
/// `SCC_CHECKMARK` in the native codepoint domain.
pub const SCC_CHECKMARK: u32 = StringControlCode::SCC_CHECKMARK.codepoint();
/// `SCC_CITY` in the native codepoint domain.
pub const SCC_CITY: u32 = StringControlCode::SCC_CITY.codepoint();
/// `SCC_CREAM` in the native codepoint domain.
pub const SCC_CREAM: u32 = StringControlCode::SCC_CREAM.codepoint();
/// `SCC_CROSS` in the native codepoint domain.
pub const SCC_CROSS: u32 = StringControlCode::SCC_CROSS.codepoint();
/// `SCC_DKBLUE` in the native codepoint domain.
pub const SCC_DKBLUE: u32 = StringControlCode::SCC_DKBLUE.codepoint();
/// `SCC_DKGREEN` in the native codepoint domain.
pub const SCC_DKGREEN: u32 = StringControlCode::SCC_DKGREEN.codepoint();
/// `SCC_DOWN_ARROW` in the native codepoint domain.
pub const SCC_DOWN_ARROW: u32 = StringControlCode::SCC_DOWN_ARROW.codepoint();
/// `SCC_GENDER_INDEX` in the native codepoint domain.
pub const SCC_GENDER_INDEX: u32 = StringControlCode::SCC_GENDER_INDEX.codepoint();
/// `SCC_GENDER_LIST` in the native codepoint domain.
pub const SCC_GENDER_LIST: u32 = StringControlCode::SCC_GENDER_LIST.codepoint();
/// `SCC_GOLD` in the native codepoint domain.
pub const SCC_GOLD: u32 = StringControlCode::SCC_GOLD.codepoint();
/// `SCC_GRAY` in the native codepoint domain.
pub const SCC_GRAY: u32 = StringControlCode::SCC_GRAY.codepoint();
/// `SCC_GREEN` in the native codepoint domain.
pub const SCC_GREEN: u32 = StringControlCode::SCC_GREEN.codepoint();
/// `SCC_LORRY` in the native codepoint domain.
pub const SCC_LORRY: u32 = StringControlCode::SCC_LORRY.codepoint();
/// `SCC_LTBLUE` in the native codepoint domain.
pub const SCC_LTBLUE: u32 = StringControlCode::SCC_LTBLUE.codepoint();
/// `SCC_LTBROWN` in the native codepoint domain.
pub const SCC_LTBROWN: u32 = StringControlCode::SCC_LTBROWN.codepoint();
/// `SCC_NEWGRF_DISCARD_WORD` in the native codepoint domain.
pub const SCC_NEWGRF_DISCARD_WORD: u32 = StringControlCode::SCC_NEWGRF_DISCARD_WORD.codepoint();
/// `SCC_NEWGRF_PRINT_BYTE_HEX` in the native codepoint domain.
pub const SCC_NEWGRF_PRINT_BYTE_HEX: u32 = StringControlCode::SCC_NEWGRF_PRINT_BYTE_HEX.codepoint();
/// `SCC_NEWGRF_PRINT_DWORD_DATE_LONG` in the native codepoint domain.
pub const SCC_NEWGRF_PRINT_DWORD_DATE_LONG: u32 =
    StringControlCode::SCC_NEWGRF_PRINT_DWORD_DATE_LONG.codepoint();
/// `SCC_NEWGRF_PRINT_DWORD_FORCE` in the native codepoint domain.
pub const SCC_NEWGRF_PRINT_DWORD_FORCE: u32 =
    StringControlCode::SCC_NEWGRF_PRINT_DWORD_FORCE.codepoint();
/// `SCC_NEWGRF_PRINT_DWORD_HEX` in the native codepoint domain.
pub const SCC_NEWGRF_PRINT_DWORD_HEX: u32 =
    StringControlCode::SCC_NEWGRF_PRINT_DWORD_HEX.codepoint();
/// `SCC_NEWGRF_PRINT_DWORD_SIGNED` in the native codepoint domain.
pub const SCC_NEWGRF_PRINT_DWORD_SIGNED: u32 =
    StringControlCode::SCC_NEWGRF_PRINT_DWORD_SIGNED.codepoint();
/// `SCC_NEWGRF_PRINT_QWORD_CURRENCY` in the native codepoint domain.
pub const SCC_NEWGRF_PRINT_QWORD_CURRENCY: u32 =
    StringControlCode::SCC_NEWGRF_PRINT_QWORD_CURRENCY.codepoint();
/// `SCC_NEWGRF_PRINT_QWORD_HEX` in the native codepoint domain.
pub const SCC_NEWGRF_PRINT_QWORD_HEX: u32 =
    StringControlCode::SCC_NEWGRF_PRINT_QWORD_HEX.codepoint();
/// `SCC_NEWGRF_PRINT_WORD_DATE_LONG` in the native codepoint domain.
pub const SCC_NEWGRF_PRINT_WORD_DATE_LONG: u32 =
    StringControlCode::SCC_NEWGRF_PRINT_WORD_DATE_LONG.codepoint();
/// `SCC_NEWGRF_PRINT_WORD_HEX` in the native codepoint domain.
pub const SCC_NEWGRF_PRINT_WORD_HEX: u32 = StringControlCode::SCC_NEWGRF_PRINT_WORD_HEX.codepoint();
/// `SCC_NEWGRF_PRINT_WORD_STATION_NAME` in the native codepoint domain.
pub const SCC_NEWGRF_PRINT_WORD_STATION_NAME: u32 =
    StringControlCode::SCC_NEWGRF_PRINT_WORD_STATION_NAME.codepoint();
/// `SCC_NEWGRF_PRINT_WORD_STRING_ID` in the native codepoint domain.
pub const SCC_NEWGRF_PRINT_WORD_STRING_ID: u32 =
    StringControlCode::SCC_NEWGRF_PRINT_WORD_STRING_ID.codepoint();
/// `SCC_NEWGRF_PRINT_WORD_VOLUME_LONG` in the native codepoint domain.
pub const SCC_NEWGRF_PRINT_WORD_VOLUME_LONG: u32 =
    StringControlCode::SCC_NEWGRF_PRINT_WORD_VOLUME_LONG.codepoint();
/// `SCC_NEWGRF_PRINT_WORD_WEIGHT_LONG` in the native codepoint domain.
pub const SCC_NEWGRF_PRINT_WORD_WEIGHT_LONG: u32 =
    StringControlCode::SCC_NEWGRF_PRINT_WORD_WEIGHT_LONG.codepoint();
/// `SCC_NEWGRF_PUSH_WORD` in the native codepoint domain.
pub const SCC_NEWGRF_PUSH_WORD: u32 = StringControlCode::SCC_NEWGRF_PUSH_WORD.codepoint();
/// `SCC_NEWGRF_ROTATE_TOP_4_WORDS` in the native codepoint domain.
pub const SCC_NEWGRF_ROTATE_TOP_4_WORDS: u32 =
    StringControlCode::SCC_NEWGRF_ROTATE_TOP_4_WORDS.codepoint();
/// `SCC_NEWGRF_STRINL` in the native codepoint domain.
pub const SCC_NEWGRF_STRINL: u32 = StringControlCode::SCC_NEWGRF_STRINL.codepoint();
/// `SCC_ORANGE` in the native codepoint domain.
pub const SCC_ORANGE: u32 = StringControlCode::SCC_ORANGE.codepoint();
/// `SCC_PLANE` in the native codepoint domain.
pub const SCC_PLANE: u32 = StringControlCode::SCC_PLANE.codepoint();
/// `SCC_PLURAL_LIST` in the native codepoint domain.
pub const SCC_PLURAL_LIST: u32 = StringControlCode::SCC_PLURAL_LIST.codepoint();
/// `SCC_POP_COLOUR` in the native codepoint domain.
pub const SCC_POP_COLOUR: u32 = StringControlCode::SCC_POP_COLOUR.codepoint();
/// `SCC_PURPLE` in the native codepoint domain.
pub const SCC_PURPLE: u32 = StringControlCode::SCC_PURPLE.codepoint();
/// `SCC_PUSH_COLOUR` in the native codepoint domain.
pub const SCC_PUSH_COLOUR: u32 = StringControlCode::SCC_PUSH_COLOUR.codepoint();
/// `SCC_RED` in the native codepoint domain.
pub const SCC_RED: u32 = StringControlCode::SCC_RED.codepoint();
/// `SCC_RIGHT_ARROW` in the native codepoint domain.
pub const SCC_RIGHT_ARROW: u32 = StringControlCode::SCC_RIGHT_ARROW.codepoint();
/// `SCC_SET_CASE` in the native codepoint domain.
pub const SCC_SET_CASE: u32 = StringControlCode::SCC_SET_CASE.codepoint();
/// `SCC_SHIP` in the native codepoint domain.
pub const SCC_SHIP: u32 = StringControlCode::SCC_SHIP.codepoint();
/// `SCC_SILVER` in the native codepoint domain.
pub const SCC_SILVER: u32 = StringControlCode::SCC_SILVER.codepoint();
/// `SCC_SMALL_DOWN_ARROW` in the native codepoint domain.
pub const SCC_SMALL_DOWN_ARROW: u32 = StringControlCode::SCC_SMALL_DOWN_ARROW.codepoint();
/// `SCC_SMALL_UP_ARROW` in the native codepoint domain.
pub const SCC_SMALL_UP_ARROW: u32 = StringControlCode::SCC_SMALL_UP_ARROW.codepoint();
/// `SCC_SUPERSCRIPT_M1` in the native codepoint domain.
pub const SCC_SUPERSCRIPT_M1: u32 = StringControlCode::SCC_SUPERSCRIPT_M1.codepoint();
/// `SCC_SWITCH_CASE` in the native codepoint domain.
pub const SCC_SWITCH_CASE: u32 = StringControlCode::SCC_SWITCH_CASE.codepoint();
/// `SCC_TINYFONT` in the native codepoint domain.
pub const SCC_TINYFONT: u32 = StringControlCode::SCC_TINYFONT.codepoint();
/// `SCC_TOWN` in the native codepoint domain.
pub const SCC_TOWN: u32 = StringControlCode::SCC_TOWN.codepoint();
/// `SCC_TRAIN` in the native codepoint domain.
pub const SCC_TRAIN: u32 = StringControlCode::SCC_TRAIN.codepoint();
/// `SCC_UP_ARROW` in the native codepoint domain.
pub const SCC_UP_ARROW: u32 = StringControlCode::SCC_UP_ARROW.codepoint();
/// `SCC_WHITE` in the native codepoint domain.
pub const SCC_WHITE: u32 = StringControlCode::SCC_WHITE.codepoint();
/// `SCC_YELLOW` in the native codepoint domain.
pub const SCC_YELLOW: u32 = StringControlCode::SCC_YELLOW.codepoint();
/// `SCC_RECORD_SEPARATOR` in the native codepoint domain.
pub const SCC_RECORD_SEPARATOR: u32 = StringControlCode::SCC_RECORD_SEPARATOR.codepoint();
/// `SCC_CONTROL_START` in the native codepoint domain.
pub const SCC_CONTROL_START: u32 = StringControlCode::SCC_CONTROL_START.codepoint();
/// `SCC_CONTROL_END` in the native codepoint domain.
pub const SCC_CONTROL_END: u32 = StringControlCode::SCC_CONTROL_END.codepoint();
/// `SCC_SPRITE_START` in the native codepoint domain.
pub const SCC_SPRITE_START: u32 = StringControlCode::SCC_SPRITE_START.codepoint();
/// `SCC_SPRITE_END` in the native codepoint domain.
pub const SCC_SPRITE_END: u32 = StringControlCode::SCC_SPRITE_END.codepoint();
/// `SCC_ENCODED` in the native codepoint domain.
pub const SCC_ENCODED: u32 = StringControlCode::SCC_ENCODED.codepoint();
/// `SCC_ENCODED_INTERNAL` in the native codepoint domain.
pub const SCC_ENCODED_INTERNAL: u32 = StringControlCode::SCC_ENCODED_INTERNAL.codepoint();
/// `SCC_ENCODED_NUMERIC` in the native codepoint domain.
pub const SCC_ENCODED_NUMERIC: u32 = StringControlCode::SCC_ENCODED_NUMERIC.codepoint();
/// `SCC_ENCODED_STRING` in the native codepoint domain.
pub const SCC_ENCODED_STRING: u32 = StringControlCode::SCC_ENCODED_STRING.codepoint();
