/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! GRF text storage and language selection. Access holds the game-state mutex,
//! including serialized GUI/game-thread calls. Text views borrow the table until
//! replacement, addition or cleanup. No table borrow spans a native callback.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::wildcard_imports,
    clippy::match_same_arms
)]
pub(crate) mod actions;
mod translation;
use crate::control_codes::SCC_NEWGRF_PRINT_WORD_STRING_ID;
use crate::game_state::GameState;
use crate::services::ByteSpan;
use crate::strings_type::{TAB_SIZE_NEWGRF, grf_string_id};
use std::ffi::c_void;
pub(crate) use translation::translate;

#[repr(C)]
pub struct Services {
    added: unsafe extern "C" fn(u32, u32, u32, u8, *const u8, usize, u32),
    empty: u32,
    language_map: unsafe extern "C" fn(u32, u8) -> *const c_void,
    map_string: unsafe extern "C" fn(u32, u32) -> u32,
    mapping: unsafe extern "C" fn(*const c_void, i32, bool) -> i32,
    max_plural_forms: u8,
    message: unsafe extern "C" fn(i32, *const u8, usize),
    num_cases: unsafe extern "C" fn() -> u8,
    num_genders: unsafe extern "C" fn() -> u8,
    plural_form: unsafe extern "C" fn(*const c_void) -> i32,
    reverse_mapping: unsafe extern "C" fn(*const c_void, i32, bool) -> i32,
    short_buffer: unsafe extern "C" fn(usize, usize),
}
impl Services {
    fn message(&self, text: &str) {
        self.log(1, text.as_bytes());
    }
    pub(crate) fn log(&self, level: i32, text: &[u8]) {
        unsafe { (self.message)(level, text.as_ptr(), text.len()) };
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDGrfTextServices" ["grf_loading"] => Services { added, empty, language_map, map_string, mapping, max_plural_forms, message, num_cases, num_genders, plural_form, reverse_mapping, short_buffer };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_text_translate(
    services: &Services,
    grfid: u32,
    language: u8,
    allow_newlines: bool,
    text: *const u8,
    length: usize,
    byte80: u32,
    output: *mut c_void,
    assign: unsafe extern "C" fn(*mut c_void, *const u8, usize),
) {
    let result = translate(
        services,
        grfid,
        language,
        allow_newlines,
        unsafe { crate::utf8::borrow(text, length) },
        byte80,
    );
    unsafe { assign(output, result.as_ptr(), result.len()) };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_text_add(
    s: &Services,
    grfid: u32,
    stringid: u32,
    mut language: u8,
    new_scheme: bool,
    allow_newlines: bool,
    text: *const u8,
    length: usize,
    default: u32,
) -> u32 {
    if !new_scheme {
        if language & 3 != 0 {
            language = 1;
        } else {
            let mut result = s.empty;
            for (mask, language_id) in [(4, 2), (8, 3), (16, 4)] {
                if language & mask != 0 {
                    result = unsafe {
                        openttd_rust_grf_text_add(
                            s,
                            grfid,
                            stringid,
                            language_id,
                            true,
                            allow_newlines,
                            text,
                            length,
                            default,
                        )
                    };
                }
            }
            return result;
        }
    }
    let index = unsafe { prepare(grfid, stringid, default) };
    if index == usize::MAX {
        return s.empty;
    }
    let newtext = translate(
        s,
        grfid,
        language,
        allow_newlines,
        unsafe { crate::utf8::borrow(text, length) },
        SCC_NEWGRF_PRINT_WORD_STRING_ID,
    );
    unsafe { publish(index, language, newtext.as_ptr(), newtext.len()) };
    let id = grf_string_id(index);
    unsafe {
        (s.added)(
            index as u32,
            grfid,
            stringid,
            language,
            newtext.as_ptr(),
            newtext.len(),
            id,
        );
    };
    id
}

struct Text {
    language: u8,
    bytes: Vec<u8>,
}
struct Entry {
    grfid: u32,
    stringid: u32,
    default: u32,
    texts: Vec<Text>,
}
static TEXTS: GameState<Vec<Entry>> = GameState::new(Vec::new());
static LANGUAGE: GameState<u8> = GameState::new(1);

fn select(language: u8, languages: impl Iterator<Item = u8>) -> Option<usize> {
    let mut fallback = None;
    for (index, lang) in languages.enumerate() {
        if lang == language {
            return Some(index);
        }
        if lang == 0x7f || (fallback.is_none() && (lang == 0 || lang == 1)) {
            fallback = Some(index);
        }
    }
    fallback
}

unsafe fn prepare(grfid: u32, stringid: u32, default: u32) -> usize {
    let texts = unsafe { &mut *TEXTS.get() };
    if let Some(index) = texts
        .iter()
        .position(|t| t.grfid == grfid && t.stringid == stringid)
    {
        return index;
    }
    if texts.len() == TAB_SIZE_NEWGRF {
        return usize::MAX;
    }
    let index = texts.len();
    texts.push(Entry {
        grfid,
        stringid,
        default,
        texts: Vec::new(),
    });
    index
}

unsafe fn publish(index: usize, language: u8, bytes: *const u8, length: usize) {
    let bytes = unsafe { crate::utf8::borrow(bytes, length) };
    let entry = unsafe { &mut (&mut *TEXTS.get())[index] };
    if let Some(text) = entry.texts.iter_mut().find(|t| t.language == language) {
        text.bytes.clear();
        text.bytes.extend_from_slice(bytes);
    } else {
        entry.texts.push(Text {
            language,
            bytes: bytes.to_vec(),
        });
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_text_id(
    grfid: u32,
    stringid: u32,
    undefined: u32,
) -> u32 {
    unsafe { &*TEXTS.get() }
        .iter()
        .position(|t| t.grfid == grfid && t.stringid == stringid)
        .map_or(undefined, grf_string_id)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_text_get(index: usize, default: *mut u32) -> ByteSpan {
    let entry = unsafe { &(&*TEXTS.get())[index] };
    assert_ne!(entry.grfid, 0);
    if let Some(index) = select(
        unsafe { *LANGUAGE.get() },
        entry.texts.iter().map(|t| t.language),
    ) {
        let text = &entry.texts[index].bytes;
        return ByteSpan {
            data: text.as_ptr(),
            size: text.len(),
        };
    }
    unsafe { default.write(entry.default) };
    ByteSpan {
        data: std::ptr::null(),
        size: 0,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_text_select(
    list: *const c_void,
    count: usize,
    language_at: unsafe extern "C" fn(*const c_void, usize) -> u8,
) -> usize {
    select(
        unsafe { *LANGUAGE.get() },
        (0..count).map(|i| unsafe { language_at(list, i) }),
    )
    .unwrap_or(usize::MAX)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_text_set_language(language: u8) {
    unsafe { *LANGUAGE.get() = language };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_text_check_language(language: u8, version: u8) -> u8 {
    let current = unsafe { *LANGUAGE.get() };
    u8::from(if version < 7 {
        language
            & match current {
                2 => 4,
                3 => 8,
                4 => 16,
                _ => 3,
            }
            != 0
    } else {
        language == current || language == 0x7f
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_text_cleanup() {
    unsafe { &mut *TEXTS.get() }.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn language_priority_and_process_state() {
        assert_eq!(select(9, [0, 1, 0x7f, 0x7f].into_iter()), Some(3));
        assert_eq!(select(1, [0x7f, 1, 0x7f].into_iter()), Some(1));
        assert_eq!(select(9, [1, 0].into_iter()), Some(0));
        assert_eq!(select(9, [2, 3].into_iter()), None);
        unsafe {
            openttd_rust_grf_text_cleanup();
            let index = prepare(123, 0xd000, 42);
            assert_eq!(index, 0);
            assert_eq!(prepare(123, 0xd000, 99), 0);
            publish(index, 2, b"German".as_ptr(), 6);
            let mut default = 0;
            assert!(
                openttd_rust_grf_text_get(index, &raw mut default)
                    .data
                    .is_null()
            );
            assert_eq!(default, 42);
            openttd_rust_grf_text_set_language(2);
        }
        std::thread::spawn(|| unsafe {
            let mut default = 0;
            let text = openttd_rust_grf_text_get(0, &raw mut default);
            assert_eq!(crate::utf8::borrow(text.data, text.size), b"German");
            assert_eq!(openttd_rust_grf_text_check_language(4, 6), 1);
            assert_eq!(openttd_rust_grf_text_check_language(1, 7), 0);
            assert_eq!(openttd_rust_grf_text_check_language(0x7f, 7), 1);
            publish(0, 2, b"Deutsch".as_ptr(), 7);
        })
        .join()
        .unwrap();
        unsafe {
            let mut default = 0;
            let text = openttd_rust_grf_text_get(0, &raw mut default);
            assert_eq!(crate::utf8::borrow(text.data, text.size), b"Deutsch");
            openttd_rust_grf_text_cleanup();
            assert_eq!(openttd_rust_grf_text_id(123, 0xd000, 2), 2);
            openttd_rust_grf_text_set_language(1);
        }
    }
}
