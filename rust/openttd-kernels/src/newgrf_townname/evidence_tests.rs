/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Action 0F parsing and town-name generation (#736 W).
//!
//! Moved from `GrfTownNames` of `tools/simulation/newgrf-loading-console.cpp`. The action
//! bytes are that fixture's, and `name-oracle.txt` is the unchanged original's log for
//! them (see its header). The reload phase is exercised by parsing again after
//! `clean_up`. Status and error leaves only record the Rust decision: the disabled
//! status of a truncated file is set by the loader's error frame, which `Inspect` keeps,
//! and founding towns with these styles stays in the console.
use super::*;
use crate::newgrf_config::GCS_DISABLED;
use crate::newgrf_loading::{Services as LoadingServices, TEST_TABLE};
use crate::test_tables::SHARED;
use std::mem::MaybeUninit;
use std::ptr::{addr_of_mut, null_mut};
use std::sync::Mutex;

const INVALID_ID: u32 = 1234;
static STATUSES: Mutex<Vec<u8>> = Mutex::new(Vec::new());
static ERRORS: Mutex<Vec<u32>> = Mutex::new(Vec::new());
static TEXTS_SEEN: Mutex<Vec<(u32, u8, u32, Vec<u8>)>> = Mutex::new(Vec::new());

extern "C" fn set_status(_: *mut c_void, status: u8) {
    STATUSES.lock().unwrap().push(status);
}
extern "C" fn error(_: *mut c_void, _: u32, message: u32, _: bool) -> *mut c_void {
    ERRORS.lock().unwrap().push(message);
    std::ptr::dangling_mut()
}
extern "C" fn grf_file_id(file: *const c_void) -> u32 {
    unsafe { file.cast::<u32>().read() }
}
extern "C" fn grf_file_version(_: *const c_void) -> u8 {
    8
}
unsafe extern "C" fn visit_text(
    _: *const c_void,
    grfid: u32,
    _: u32,
    language: u8,
    _: u32,
    returned: u32,
    text: ByteSpan,
) {
    let bytes = unsafe { std::slice::from_raw_parts(text.data, text.size) }.to_vec();
    TEXTS_SEEN.lock().unwrap().push((grfid, language, returned, bytes));
}
extern "C" fn random(_: *mut c_void) -> u32 {
    unreachable!()
}
extern "C" fn industry(_: i32, _: i32, _: *mut u32) -> u32 {
    unreachable!()
}

fn text(prob: u8, value: &[u8]) -> Vec<u8> {
    [&[prob], value, &[0]].concat()
}
fn grfid(id: &[u8; 4]) -> u32 {
    u32::from_le_bytes(*id)
}
fn actions_a() -> Vec<Vec<u8>> {
    vec![
        [&[0x01, 1, 2, 0, 8][..], &text(1, b"Al"), &text(127, b"Be")].concat(),
        // A repeated definition appends a second part list.
        [&[0x01, 1, 1, 8, 4][..], &text(5, b"ton")].concat(),
        [&[0x02, 1, 3, 12, 0][..], &text(0, b"X"), &text(3, b"Y"), &text(0, b"Z")].concat(),
        // Two localized names; bit 7 of the second language is cleared. A 32-bit count
        // follows the x86 shift masking of the original GB.
        [
            &[0x83, 0x7F][..],
            b"Alpha\0\x81Alpha US\0\0",
            &[2, 2, 16, 2, 0x81, 0x01, 0x82, 0x02, 1, 30, 32],
            &text(1, b" Hill"),
        ]
        .concat(),
        [&[0x84, 0x7F][..], b"Beta\0\0", &[1, 2, 0, 16, 0xFF, 0x03], &text(1, b"Q")].concat(),
    ]
}
fn actions_bad() -> Vec<Vec<u8>> {
    vec![
        [&[0x05, 1, 1, 0, 1][..], &text(1, b"ok")].concat(),
        [
            &[0x80, 0x7F][..],
            b"Bad\0\0",
            &[1, 2, 0, 1],
            &text(1, b"ok"),
            &[0x81, 0x10],
        ]
        .concat(),
    ]
}
fn actions_b() -> Vec<Vec<u8>> {
    vec![[&[0x85, 0x7F][..], b"Gamma\0\0", &[1, 1, 0, 1], &text(1, b"Gam\x9e")].concat()]
}
fn actions_short() -> Vec<Vec<u8>> {
    // The reference byte of the last part is missing.
    vec![
        [
            &[0x81, 0x7F][..],
            b"Short\0\0",
            &[2, 1, 0, 3],
            &text(2, b"Short"),
            &[3, 4, 4],
            &text(1, b"ort"),
            &[0x82],
        ]
        .concat(),
    ]
}
fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut out, b| {
        write!(out, "{b:02X}").unwrap();
        out
    })
}

struct Fixture {
    shared: crate::services::Services,
    text: crate::newgrf_text::Services,
    engine: Box<MaybeUninit<crate::engine_loading::Services>>,
    loading: LoadingServices,
    ids: Vec<Box<u32>>,
}
impl Fixture {
    fn new() -> Box<Self> {
        let mut shared = SHARED;
        shared.grf_file_id = grf_file_id;
        shared.grf_file_version = grf_file_version;
        let mut f = Box::new(Self {
            shared,
            text: crate::newgrf_text::test_services(),
            engine: Box::new(MaybeUninit::zeroed()),
            loading: TEST_TABLE,
            ids: Vec::new(),
        });
        // Only `shared` of this zeroed table is ever read, through raw pointers.
        unsafe {
            addr_of_mut!((*f.engine.as_mut_ptr()).shared).write(&raw const f.shared);
        }
        f.loading.engine = f.engine.as_ptr();
        f.loading.text = &raw const f.text;
        f.loading.set_status = set_status;
        f.loading.error = error;
        f.loading.invalid_id = INVALID_ID;
        f
    }
    /// One file's actions in order; a read failure ends the file, as the loader's error frame.
    fn load(&mut self, id: &[u8; 4], actions: &[Vec<u8>]) -> bool {
        let mut grf = Box::new(grfid(id));
        let state = newgrf_loading::openttd_rust_grf_execution();
        unsafe {
            (*state).grffile = (&raw mut *grf).cast();
            (*state).skip_sprites = 0;
        }
        self.ids.push(grf);
        for action in actions {
            let mut reader = ByteReader::new(action.as_ptr(), action.len());
            if unsafe { feature_town_name(&self.loading, &mut reader) }.is_err() {
                return false;
            }
            // Other files' actions are skipped after a disable; this file has none left.
            if newgrf_loading::is_disabled() {
                return true;
            }
        }
        true
    }
    fn load_all(&mut self) {
        unsafe {
            clean_up();
            crate::newgrf_text::openttd_rust_grf_text_cleanup();
        }
        assert!(self.load(b"TNA5", &actions_a()));
        assert!(self.load(b"TNX5", &actions_bad()));
        assert!(self.load(b"TNB5", &actions_b()));
        assert!(!self.load(b"TNS5", &actions_short()));
        unsafe { init_generator_names() };
    }
    fn name(&self, id: u32, gen_: u16, seed: u32) -> Vec<u8> {
        let mut out = Vec::new();
        unsafe { generate(&self.shared, &mut |part| out.extend_from_slice(part), id, gen_, seed) };
        out
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        unsafe {
            clean_up();
            crate::newgrf_text::openttd_rust_grf_text_cleanup();
            let state = newgrf_loading::openttd_rust_grf_execution();
            (*state).grffile = null_mut();
            (*state).skip_sprites = 0;
        }
    }
}

fn names() -> Vec<u32> {
    let mut size = 0;
    let data = unsafe { openttd_rust_grf_townname_list(&raw mut size) };
    unsafe { std::slice::from_raw_parts(data, size) }.to_vec()
}

#[test]
fn action_0f_parsing_and_generation_match_the_original() {
    let _guard = crate::town::tests::GLOBAL_LOCK.lock().unwrap();
    STATUSES.lock().unwrap().clear();
    ERRORS.lock().unwrap().clear();
    let mut f = Fixture::new();
    f.load_all();
    let first = names();

    // Style order, then the fallback past the last style.
    assert_eq!(first.len(), 4);
    let ids = [b"TNA5", b"TNA5", b"TNB5", b"TNS5"].map(grfid);
    for (gen_, id) in ids.into_iter().enumerate() {
        assert_eq!(unsafe { get_id(gen_ as u16) }, id);
    }
    assert_eq!(unsafe { get_type(1) }, 1);
    assert_eq!(unsafe { get_id(4) }, 0);
    assert_eq!(unsafe { get_type(4) }, SPECSTR_TOWNNAME_START as u16);

    let (a, b, s, x) = (grfid(b"TNA5"), grfid(b"TNB5"), grfid(b"TNS5"), grfid(b"TNX5"));
    // A repeated definition, the recursive final definition and the second file.
    assert_eq!(f.name(a, 0, 0), b"Y Hill");
    assert_eq!(f.name(a, 0, u32::MAX), b"Alton Hill");
    assert_eq!(f.name(a, 0, 0x0003_0000), b"Beton Hill");
    assert_eq!(f.name(a, 1, 0), b"Q");
    assert_eq!(f.name(a, 1, 0xFFFF), b"Y Hill");
    assert_eq!(f.name(b, 0, 0), "Gam\u{20ac}".as_bytes());
    // The invalid reference deleted its entry and disabled its file.
    assert!(!unsafe { exists(x) });
    assert!(f.name(x, 0, 0).is_empty());
    assert_eq!(*STATUSES.lock().unwrap(), [GCS_DISABLED]);
    assert_eq!(*ERRORS.lock().unwrap(), [INVALID_ID]);
    // The read failure of the truncated file keeps the parts read before it.
    assert_eq!(f.name(s, 0, 0), b"Shortort");

    check_oracle(&f, &first);

    // The second load of `ReloadNewGRFData` behaves as the first.
    f.load_all();
    assert_eq!(names().len(), first.len());
    check_oracle(&f, &names());
}

fn check_oracle(f: &Fixture, list: &[u32]) {
    let builtin = crate::services::fixture(null_mut(), random, industry);
    let mut styles = 0;
    let mut samples = 0;
    TEXTS_SEEN.lock().unwrap().clear();
    unsafe { crate::newgrf_text::visit(std::ptr::null(), visit_text) };
    let seen = std::mem::take(&mut *TEXTS_SEEN.lock().unwrap());
    for row in include_str!("name-oracle.txt").lines().filter(|l| !l.starts_with('#')) {
        let field: Vec<&str> = row.split(' ').collect();
        let gen_: u16 = field[1].parse().unwrap();
        if field[0] == "style" {
            styles += 1;
            let id = u32::from_str_radix(field[2], 16).unwrap();
            assert_eq!(unsafe { get_id(gen_) }, id, "{row}");
            assert_eq!(u32::from(unsafe { get_type(gen_) }), field[3].parse::<u32>().unwrap());
            if field[4] == "true" {
                // The console's default language selects the US name of the first style.
                let name = list[usize::from(gen_)];
                let language = if gen_ == 0 { 1 } else { 0x7F };
                let matched: Vec<_> = seen
                    .iter()
                    .filter(|(g, l, r, _)| *g == id && *l == language && *r == name)
                    .collect();
                assert_eq!(matched.len(), 1, "{row}");
                assert_eq!(hex(&matched[0].3), field[5], "{row}");
            }
        } else {
            samples += 1;
            let seed = u32::from_str_radix(field[2], 16).unwrap();
            let expected = field.get(3).copied().unwrap_or("");
            let actual = if usize::from(gen_) < list.len() {
                let id = unsafe { get_id(gen_) };
                let kind = unsafe { get_type(gen_) };
                hex(&f.name(id, kind, seed))
            } else {
                // The built-in English generator, which the fallback selects.
                let owner = unsafe { crate::townname::openttd_rust_townname_generate(&raw const builtin, 0, seed) };
                let text = hex(unsafe { &*owner }.as_bytes());
                unsafe { crate::townname::openttd_rust_townname_destroy(owner) };
                text
            };
            assert_eq!(actual, expected, "{row}");
        }
    }
    assert_eq!((styles, samples), (5, 200));
}
