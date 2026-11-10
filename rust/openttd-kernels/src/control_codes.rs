/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared `table/control_codes.h` constants used by GRF text translation.
/// Switch to large font
pub const SCC_BIGFONT: u32 = 0xe006;
/// `SCC_BLACK` from `table/control_codes.h`.
pub const SCC_BLACK: u32 = 0xe052;
/// `SCC_BLUE` from `table/control_codes.h`.
pub const SCC_BLUE: u32 = 0xe042;
/// `SCC_BROWN` from `table/control_codes.h`.
pub const SCC_BROWN: u32 = 0xe04d;
/// `SCC_BUS` from `table/control_codes.h`.
pub const SCC_BUS: u32 = 0xe2b6;
/// `SCC_CHECKMARK` from `table/control_codes.h`.
pub const SCC_CHECKMARK: u32 = 0xe2ac;
/// `SCC_CITY` from `table/control_codes.h`.
pub const SCC_CITY: u32 = 0xe29c;
/// `SCC_CREAM` from `table/control_codes.h`.
pub const SCC_CREAM: u32 = 0xe04c;
/// `SCC_CROSS` from `table/control_codes.h`.
pub const SCC_CROSS: u32 = 0xe2ad;
/// `SCC_DKBLUE` from `table/control_codes.h`.
pub const SCC_DKBLUE: u32 = 0xe051;
/// `SCC_DKGREEN` from `table/control_codes.h`.
pub const SCC_DKGREEN: u32 = 0xe04b;
/// `SCC_DOWN_ARROW` from `table/control_codes.h`.
pub const SCC_DOWN_ARROW: u32 = 0xe2aa;
/// `SCC_GENDER_INDEX` from `table/control_codes.h`.
pub const SCC_GENDER_INDEX: u32 = 0xe03e;
/// `SCC_GENDER_LIST` from `table/control_codes.h`.
pub const SCC_GENDER_LIST: u32 = 0xe03d;
/// `SCC_GOLD` from `table/control_codes.h`.
pub const SCC_GOLD: u32 = 0xe044;
/// `SCC_GRAY` from `table/control_codes.h`.
pub const SCC_GRAY: u32 = 0xe050;
/// `SCC_GREEN` from `table/control_codes.h`.
pub const SCC_GREEN: u32 = 0xe049;
/// `SCC_LORRY` from `table/control_codes.h`.
pub const SCC_LORRY: u32 = 0xe2b5;
/// `SCC_LTBLUE` from `table/control_codes.h`.
pub const SCC_LTBLUE: u32 = 0xe04f;
/// `SCC_LTBROWN` from `table/control_codes.h`.
pub const SCC_LTBROWN: u32 = 0xe047;
/// 85: Discard the next two bytes
pub const SCC_NEWGRF_DISCARD_WORD: u32 = 0xe072;
/// 9A 06: Read 1 byte from the stack and print it as hex
pub const SCC_NEWGRF_PRINT_BYTE_HEX: u32 = 0xe063;
/// 9A 16: Read 4 bytes from the stack as base 0 date
pub const SCC_NEWGRF_PRINT_DWORD_DATE_LONG: u32 = 0xe067;
/// 9A 21: Read 4 bytes from the stack as unsigned force
pub const SCC_NEWGRF_PRINT_DWORD_FORCE: u32 = 0xe070;
/// 9A 08: Read 4 bytes from the stack and print it as hex
pub const SCC_NEWGRF_PRINT_DWORD_HEX: u32 = 0xe065;
/// 7B: Read 4 bytes from the stack
pub const SCC_NEWGRF_PRINT_DWORD_SIGNED: u32 = 0xe056;
/// 9A 01: Read 8 bytes from the stack as currency
pub const SCC_NEWGRF_PRINT_QWORD_CURRENCY: u32 = 0xe062;
/// 9A 0B: Read 8 bytes from the stack and print it as hex
pub const SCC_NEWGRF_PRINT_QWORD_HEX: u32 = 0xe066;
/// 82: Read 2 bytes from the stack as base 1920 date
pub const SCC_NEWGRF_PRINT_WORD_DATE_LONG: u32 = 0xe05c;
/// 9A 07: Read 2 bytes from the stack and print it as hex
pub const SCC_NEWGRF_PRINT_WORD_HEX: u32 = 0xe064;
/// 9A 0C: Read 2 bytes from the stack as station name
pub const SCC_NEWGRF_PRINT_WORD_STATION_NAME: u32 = 0xe061;
/// 81: Read 2 bytes from the stack as String ID
pub const SCC_NEWGRF_PRINT_WORD_STRING_ID: u32 = 0xe05b;
/// 87: Read 2 bytes from the stack as long signed volume
pub const SCC_NEWGRF_PRINT_WORD_VOLUME_LONG: u32 = 0xe05f;
/// 9A 0D: Read 2 bytes from the stack as long unsigned weight
pub const SCC_NEWGRF_PRINT_WORD_WEIGHT_LONG: u32 = 0xe060;
/// 9A 03: Pushes 2 bytes onto the stack
pub const SCC_NEWGRF_PUSH_WORD: u32 = 0xe071;
/// 86: Rotate the top 4 words of the stack (W4 W1 W2 W3)
pub const SCC_NEWGRF_ROTATE_TOP_4_WORDS: u32 = 0xe073;
/// Inline another string at the current position, StringID is encoded in the string
pub const SCC_NEWGRF_STRINL: u32 = 0xe074;
/// `SCC_ORANGE` from `table/control_codes.h`.
pub const SCC_ORANGE: u32 = 0xe048;
/// `SCC_PLANE` from `table/control_codes.h`.
pub const SCC_PLANE: u32 = 0xe2b7;
/// `SCC_PLURAL_LIST` from `table/control_codes.h`.
pub const SCC_PLURAL_LIST: u32 = 0xe03c;
/// `SCC_POP_COLOUR` from `table/control_codes.h`.
pub const SCC_POP_COLOUR: u32 = 0xe055;
/// `SCC_PURPLE` from `table/control_codes.h`.
pub const SCC_PURPLE: u32 = 0xe046;
/// `SCC_PUSH_COLOUR` from `table/control_codes.h`.
pub const SCC_PUSH_COLOUR: u32 = 0xe054;
/// `SCC_RED` from `table/control_codes.h`.
pub const SCC_RED: u32 = 0xe045;
/// `SCC_RIGHT_ARROW` from `table/control_codes.h`.
pub const SCC_RIGHT_ARROW: u32 = 0xe2af;
/// `SCC_SET_CASE` from `table/control_codes.h`.
pub const SCC_SET_CASE: u32 = 0xe040;
/// `SCC_SHIP` from `table/control_codes.h`.
pub const SCC_SHIP: u32 = 0xe2b8;
/// `SCC_SILVER` from `table/control_codes.h`.
pub const SCC_SILVER: u32 = 0xe043;
/// `SCC_SMALL_DOWN_ARROW` from `table/control_codes.h`.
pub const SCC_SMALL_DOWN_ARROW: u32 = 0xe2bd;
/// `SCC_SMALL_UP_ARROW` from `table/control_codes.h`.
pub const SCC_SMALL_UP_ARROW: u32 = 0xe2bc;
/// `SCC_SUPERSCRIPT_M1` from `table/control_codes.h`.
pub const SCC_SUPERSCRIPT_M1: u32 = 0xe2b9;
/// `SCC_SWITCH_CASE` from `table/control_codes.h`.
pub const SCC_SWITCH_CASE: u32 = 0xe041;
/// Switch to small font
pub const SCC_TINYFONT: u32 = 0xe005;
/// `SCC_TOWN` from `table/control_codes.h`.
pub const SCC_TOWN: u32 = 0xe29b;
/// `SCC_TRAIN` from `table/control_codes.h`.
pub const SCC_TRAIN: u32 = 0xe2b4;
/// `SCC_UP_ARROW` from `table/control_codes.h`.
pub const SCC_UP_ARROW: u32 = 0xe2a0;
/// `SCC_WHITE` from `table/control_codes.h`.
pub const SCC_WHITE: u32 = 0xe04e;
/// `SCC_YELLOW` from `table/control_codes.h`.
pub const SCC_YELLOW: u32 = 0xe04a;

/// Record separator for encoded strings.
pub const SCC_RECORD_SEPARATOR: u32 = 0x1e;
/// First nonprintable string control code.
pub const SCC_CONTROL_START: u32 = 0xe000;
/// Last nonprintable string control code.
pub const SCC_CONTROL_END: u32 = 0xe1ff;
/// First special printable sprite symbol; these map to original glyphs.
pub const SCC_SPRITE_START: u32 = 0xe200;
/// Last special printable sprite symbol.
pub const SCC_SPRITE_END: u32 = SCC_SPRITE_START + 0xff;

// All SCC_ENCODED* control codes must have stable ids as they are stored in
// strings that are saved in savegames.
/// Encoded string marker and sub-string parameter.
pub const SCC_ENCODED: u32 = SCC_CONTROL_START;
/// Encoded text from OpenTTD.
pub const SCC_ENCODED_INTERNAL: u32 = SCC_ENCODED + 1;
/// Encoded numeric parameter.
pub const SCC_ENCODED_NUMERIC: u32 = SCC_ENCODED + 2;
/// Encoded string parameter.
pub const SCC_ENCODED_STRING: u32 = SCC_ENCODED + 3;
