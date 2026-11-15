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
use crate::debug::{Bytes, grf_msg};
use crate::game_state::GameState;
use crate::services::ByteSpan;
use crate::strings_type::{TAB_SIZE_NEWGRF, grf_string_id};
use std::ffi::c_void;
#[cfg(test)]
pub(crate) use translation::tests::services as test_services;
pub(crate) use translation::translate;

#[repr(C)]
pub struct Services {
    empty: u32,
    language_map: unsafe extern "C" fn(u32, u8) -> *const c_void,
    legacy_string: unsafe extern "C" fn(u32) -> u32,
    mapping: unsafe extern "C" fn(*const c_void, i32, bool) -> i32,
    max_plural_forms: u8,
    num_cases: unsafe extern "C" fn() -> u8,
    num_genders: unsafe extern "C" fn() -> u8,
    plural_form: unsafe extern "C" fn(*const c_void) -> i32,
    reverse_mapping: unsafe extern "C" fn(*const c_void, i32, bool) -> i32,
    undefined: u32,
}
impl Services {
    /// `GrfMsg(1, text)` of the original string translation diagnostics.
    #[allow(clippy::unused_self)] // Translation callers reach the message through their table.
    fn message(&self, text: &str) {
        grf_msg!(1, "{}", text);
    }
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDGrfTextServices" ["grf_loading"] => Services { empty, language_map, legacy_string, mapping, max_plural_forms, num_cases, num_genders, plural_form, reverse_mapping, undefined };
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
    grf_msg!(
        3,
        "Added 0x{:X} grfid {:08X} string 0x{:X} lang 0x{:X} string '{}' ({:X})",
        index as u32,
        grfid,
        stringid,
        language,
        Bytes(&newtext),
        id
    );
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

/// `MapGRFStringID`: native legacy mappings cannot reenter the GRF table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_text_map(s: &Services, grfid: u32, stringid: u32) -> u32 {
    if (0xd000..0x10000).contains(&stringid) {
        let stringid = if stringid < 0xd800 {
            // TTDPatch adds 0x400 for inline callback text references.
            stringid & !0x400
        } else {
            stringid
        };
        unsafe { openttd_rust_grf_text_id(grfid, stringid, s.undefined) }
    } else {
        unsafe { (s.legacy_string)(stringid) }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_text_get(
    shared: &crate::services::Services,
    index: usize,
    default: *mut u32,
) -> ByteSpan {
    crate::movement::native_assert(shared, index < unsafe { &*TEXTS.get() }.len());
    crate::movement::native_assert(shared, unsafe {
        (*(*TEXTS.get()).as_ptr().add(index)).grfid != 0
    });
    // Native operator[] is unchecked when assertions are disabled. Reacquire the
    // entry after a returning assertion handler, without a table borrow across it.
    let entry = unsafe { &*(*TEXTS.get()).as_ptr().add(index) };
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

/// Prefix of a retained native `GRFText`. The native stride includes its opaque
/// `std::string` tail. Access only `langid` through raw pointers, without a borrow
/// or a Rust-sized slice of the native records. C++ checks the prefix offset.
#[repr(C)]
pub struct GrfText {
    langid: u8,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_text_select(
    list: *const GrfText,
    count: usize,
    stride: usize,
) -> usize {
    select(
        unsafe { *LANGUAGE.get() },
        (0..count).map(|i| unsafe { (&raw const (*list.byte_add(i * stride)).langid).read() }),
    )
    .unwrap_or(usize::MAX)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_text_set_language(language: u8) {
    unsafe { *LANGUAGE.get() = language };
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_text_check_language_abi(language: u8, version: u8) -> u8 {
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

/// `CleanUpStrings`: house cleaning. Remove all strings.
/// # Safety
/// Serialized game state; no borrow of the text table is live.
pub(crate) unsafe fn clean_up_strings() {
    unsafe { &mut *TEXTS.get() }.clear();
}

/// Fixture-only table observation; the visitor may look up strings but cannot mutate the table.
#[cfg(any(test, feature = "native-test-support"))]
pub(crate) unsafe fn visit(
    context: *const c_void,
    visitor: unsafe extern "C" fn(*const c_void, u32, u32, u8, u32, u32, ByteSpan),
) -> u8 {
    let count = unsafe { &*TEXTS.get() }.len();
    for index in 0..count {
        let languages = unsafe { &(&*TEXTS.get())[index] }.texts.len();
        for language in 0..languages {
            let (grfid, stringid, lang, default, bytes) = {
                let entry = unsafe { &(&*TEXTS.get())[index] };
                let text = &entry.texts[language];
                (
                    entry.grfid,
                    entry.stringid,
                    text.language,
                    entry.default,
                    ByteSpan {
                        data: text.bytes.as_ptr(),
                        size: text.bytes.len(),
                    },
                )
            };
            unsafe {
                visitor(
                    context,
                    grfid,
                    stringid,
                    lang,
                    default,
                    grf_string_id(index),
                    bytes,
                );
            };
        }
    }
    unsafe { *LANGUAGE.get() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static ASSERTIONS: AtomicUsize = AtomicUsize::new(0);

    extern "C" fn unused_random(_: *mut c_void) -> u32 {
        std::process::abort()
    }
    extern "C" fn unused_industry(_: i32, _: i32, _: *mut u32) -> u32 {
        std::process::abort()
    }
    fn shared() -> crate::services::Services {
        crate::services::fixture(std::ptr::null_mut(), unused_random, unused_industry)
    }
    extern "C" fn returning_assert(condition: bool) {
        assert!(!condition);
        unsafe {
            match ASSERTIONS.fetch_add(1, Ordering::SeqCst) {
                0 => {
                    assert_eq!(prepare(0, 0xd000, 42), 0);
                }
                1 => {
                    (&mut *TEXTS.get())[0].grfid = 123;
                    publish(0, 1, b"assertion".as_ptr(), 9);
                    openttd_rust_grf_text_set_language(1);
                }
                _ => std::process::abort(),
            }
        }
    }

    #[repr(C)]
    struct NativeText {
        language: u8,
        opaque: [usize; 4],
    }
    #[test]
    fn language_priority_and_process_state() {
        let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
        assert_eq!(select(9, [0, 1, 0x7f, 0x7f].into_iter()), Some(3));
        assert_eq!(select(1, [0x7f, 1, 0x7f].into_iter()), Some(1));
        assert_eq!(select(9, [1, 0].into_iter()), Some(0));
        assert_eq!(select(9, [2, 3].into_iter()), None);
        unsafe {
            clean_up_strings();
            let index = prepare(123, 0xd000, 42);
            assert_eq!(index, 0);
            assert_eq!(prepare(123, 0xd000, 99), 0);
            let services = translation::tests::services();
            for (local, expected) in [
                (0xd000, grf_string_id(0)),
                (0xd400, grf_string_id(0)),
                (0xd800, 2),
                (0xffff, 2),
                (0x10000, 0x30000),
                (1, 0x20001),
            ] {
                assert_eq!(openttd_rust_grf_text_map(&services, 123, local), expected);
            }
            let inline = translate(&services, 123, 1, false, b"\x81\x00\xd4", 0);
            assert_eq!(inline, b"\xee\x81\xb4\xf0\xa0\x80\x80");
            let native = [2, 3].map(|language| NativeText {
                language,
                opaque: [0; 4],
            });
            let ptr = native.as_ptr().cast::<GrfText>();
            let stride = std::mem::size_of::<NativeText>();
            assert_eq!(
                openttd_rust_grf_text_select(ptr, native.len(), stride),
                usize::MAX
            );
            openttd_rust_grf_text_set_language(3);
            assert_eq!(openttd_rust_grf_text_select(ptr, native.len(), stride), 1);
            assert_eq!(
                openttd_rust_grf_text_select(std::ptr::null(), 0, stride),
                usize::MAX
            );
            openttd_rust_grf_text_set_language(1);
            publish(index, 2, b"German".as_ptr(), 6);
            let mut default = 0;
            assert!(
                openttd_rust_grf_text_get(&shared(), index, &raw mut default)
                    .data
                    .is_null()
            );
            assert_eq!(default, 42);
            openttd_rust_grf_text_set_language(2);
        }
        std::thread::spawn(|| unsafe {
            let mut default = 0;
            let text = openttd_rust_grf_text_get(&shared(), 0, &raw mut default);
            assert_eq!(crate::utf8::borrow(text.data, text.size), b"German");
            assert_eq!(openttd_rust_grf_text_check_language_abi(4, 6), 1);
            assert_eq!(openttd_rust_grf_text_check_language_abi(1, 7), 0);
            assert_eq!(openttd_rust_grf_text_check_language_abi(0x7f, 7), 1);
            publish(0, 2, b"Deutsch".as_ptr(), 7);
        })
        .join()
        .unwrap();
        unsafe {
            let mut default = 0;
            let text = openttd_rust_grf_text_get(&shared(), 0, &raw mut default);
            assert_eq!(crate::utf8::borrow(text.data, text.size), b"Deutsch");
            clean_up_strings();
            assert_eq!(openttd_rust_grf_text_id(123, 0xd000, 2), 2);
            let mut movement = crate::train_reservation::tests::movement_table();
            movement.native_assert = returning_assert;
            let mut shared = shared();
            shared.movement = &raw const movement;
            let text = openttd_rust_grf_text_get(&shared, 0, &raw mut default);
            assert_eq!(crate::utf8::borrow(text.data, text.size), b"assertion");
            assert_eq!(ASSERTIONS.load(Ordering::SeqCst), 2);
            clean_up_strings();
            openttd_rust_grf_text_set_language(1);
        }
    }
}
