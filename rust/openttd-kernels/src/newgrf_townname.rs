/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `newgrf_townname.cpp` and Action 0F (`newgrf/newgrf_actf.cpp`'s `FeatureTownName`):
//! implementation of Action 0F "universal holder" structure and functions, a list of
//! townname generators holding everything that the `NewGRF` action 0F will send over.
//!
//! Both lists are the original's file statics, process-wide; every access holds the
//! game-state mutex. Borrows of the lists end before each native string or log service, none
//! of which reenters this module.
use crate::bitmath_func::{gb, has_bit};
use crate::control_codes::SCC_NEWGRF_PRINT_WORD_STRING_ID;
use crate::debug::{Bytes, grf_msg};
use crate::game_state::GameState;
use crate::newgrf::GRFFileRef;
use crate::newgrf_bytereader::{Cursor as ByteReader, ReadBounds};
use crate::newgrf_loading::{self, Services};
use crate::newgrf_text::{AddGRFString, add_grf_string};
use crate::newgrf_text_type::GRFStringID;
use crate::services::ByteSpan;
use crate::strings_type::{SPECSTR_TOWNNAME_START, StringID};
use crate::table::strings::STR_UNDEFINED;
use std::ffi::c_void;
use std::ptr;

#[allow(
    unsafe_code,
    reason = "the tests' fixture tables and the process-wide lists"
)]
#[cfg(test)]
mod evidence_tests;

#[derive(Default)]
struct NamePart {
    /// If probability bit 7 is clear
    text: Vec<u8>,
    /// If probability bit 7 is set
    id: u8,
    /// The relative probability of the following name to appear in the bottom 7 bits.
    prob: u8,
}

#[derive(Default)]
struct NamePartList {
    /// Start of random seed bits to use.
    bitstart: u8,
    /// Number of bits of random seed to use.
    bitcount: u8,
    /// Total probability of all parts.
    maxprob: u16,
    /// List of parts to choose from.
    parts: Vec<NamePart>,
}

struct TownNameStyle {
    /// String ID of this town name style.
    name: StringID,
    /// Index within partlist for this town name style.
    id: u8,
}

pub(crate) struct GRFTownName {
    /// GRF ID of `NewGRF`.
    grfid: u32,
    /// Style names defined by the Town Name `NewGRF`.
    styles: Vec<TownNameStyle>,
    /// Lists of town name parts.
    partlists: [Vec<NamePartList>; GRFTownName::MAX_LISTS],
}
impl GRFTownName {
    /// Maximum number of town name lists that can be defined per GRF.
    const MAX_LISTS: usize = 128;
}

/// `_grf_townnames`.
static GRF_TOWNNAMES: GameState<Vec<GRFTownName>> = GameState::new(Vec::new());
/// `_grf_townname_names`.
static GRF_TOWNNAME_NAMES: GameState<Vec<StringID>> = GameState::new(Vec::new());

/// `GetGRFTownName`: the entry of `grfid`, or null.
///
/// # Safety
/// The caller holds the game-state mutex, and the pointer is used only until the list next
/// changes.
#[allow(unsafe_code, reason = "the process-wide list")]
pub(crate) unsafe fn get_grf_town_name(grfid: u32) -> *mut GRFTownName {
    // SAFETY: The caller's contract; the borrow ends here.
    let list = unsafe { &mut *GRF_TOWNNAMES.get() };
    list.iter_mut()
        .find(|t| t.grfid == grfid)
        .map_or(ptr::null_mut(), ptr::from_mut)
}

/// `AddGRFTownName`.
///
/// # Safety
/// As [`get_grf_town_name`].
#[allow(unsafe_code, reason = "the process-wide list")]
unsafe fn add_grf_town_name(grfid: u32) -> *mut GRFTownName {
    let mut t = unsafe { get_grf_town_name(grfid) };
    if t.is_null() {
        // SAFETY: The caller's contract; the borrow ends here.
        let list = unsafe { &mut *GRF_TOWNNAMES.get() };
        let index = list.len();
        list.push(GRFTownName {
            grfid,
            styles: Vec::new(),
            partlists: [const { Vec::new() }; GRFTownName::MAX_LISTS],
        });
        t = &raw mut list[index];
    }
    t
}

/// `DelGRFTownName`.
///
/// # Safety
/// As [`get_grf_town_name`].
#[allow(unsafe_code, reason = "the process-wide list")]
unsafe fn del_grf_town_name(grfid: u32) {
    // SAFETY: The caller's contract; the borrow ends here.
    let list = unsafe { &mut *GRF_TOWNNAMES.get() };
    // The original erases the `find` result unchecked: its only caller has just added the
    // entry, so it is found.
    let index = list.iter().position(|t| t.grfid == grfid).unwrap();
    list.remove(index);
}

/// `RandomPart`. The original's `assert(t != nullptr)` holds by the reference type.
fn random_part(builder: &mut dyn FnMut(&[u8]), t: &GRFTownName, seed: u32, id: u8) {
    for partlist in &t.partlists[usize::from(id)] {
        let count = partlist.bitcount;
        let mut maxprob = partlist.maxprob;
        let r = gb(seed, partlist.bitstart, count)
            .wrapping_mul(u32::from(maxprob))
            .wrapping_shr(u32::from(count));
        for part in &partlist.parts {
            #[allow(
                clippy::cast_possible_truncation,
                reason = "GB(part.prob, 0, 7) subtracted from the uint16_t maxprob (newgrf_townname.cpp)"
            )]
            {
                maxprob = maxprob.wrapping_sub(gb(u32::from(part.prob), 0, 7) as u16);
            }
            if u32::from(maxprob) > r {
                continue;
            }
            if has_bit(part.prob, 7) {
                random_part(builder, t, seed, part.id);
            } else {
                builder(&part.text);
            }
            break;
        }
    }
}

/// `GRFTownNameGenerate`, appending each selected part to `builder`.
///
/// # Safety
/// As [`get_grf_town_name`]; `builder` does not reach the lists.
#[allow(
    unsafe_code,
    reason = "the process-wide list and the unchecked style index"
)]
pub(crate) unsafe fn grf_town_name_generate(
    builder: &mut dyn FnMut(&[u8]),
    grfid: u32,
    gen_: u16,
    seed: u32,
) {
    let t = unsafe { get_grf_town_name(grfid) };
    if !t.is_null() {
        // SAFETY: The list does not change while the name is generated.
        let t = unsafe { &*t };
        crate::services_assert::assert_with(|| usize::from(gen_) < t.styles.len());
        // C++ operator[] is unchecked when its native assertion is disabled.
        let id = unsafe { (*t.styles.as_ptr().add(usize::from(gen_))).id };
        random_part(builder, t, seed, id);
    }
}

/// `GRFTownNameGenerate` for C++: appends each selected part to the native builder through
/// `append`.
///
/// # Safety
/// As [`grf_town_name_generate`]; `append` appends to `builder`.
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_townname_generate(
    builder: *mut c_void,
    append: unsafe extern "C" fn(*mut c_void, ByteSpan),
    grfid: u32,
    gen_: u16,
    seed: u32,
) {
    unsafe {
        grf_town_name_generate(
            &mut |text| {
                append(
                    builder,
                    ByteSpan {
                        data: text.as_ptr(),
                        size: text.len(),
                    },
                );
            },
            grfid,
            gen_,
            seed,
        );
    }
}

/// `InitGRFTownGeneratorNames`: allocate memory for the `NewGRF` town names.
///
/// # Safety
/// As [`get_grf_town_name`].
#[allow(unsafe_code, reason = "the process-wide lists")]
pub(crate) unsafe fn init_grf_town_generator_names() {
    // SAFETY: The caller's contract; the borrows end here.
    let (names, townnames) = unsafe { (&mut *GRF_TOWNNAME_NAMES.get(), &*GRF_TOWNNAMES.get()) };
    names.clear();
    for t in townnames {
        for style in &t.styles {
            names.push(style.name);
        }
    }
}

/// `GetGRFTownNameList`: valid until the next `NewGRF` load.
///
/// # Safety
/// As [`get_grf_town_name`]; `size` is writable.
#[allow(unsafe_code, reason = "export")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_townname_list(size: *mut usize) -> *const StringID {
    // SAFETY: The caller's contract.
    let names = unsafe { get_grf_town_name_list() };
    unsafe { size.write(names.len()) };
    names.as_ptr()
}

/// `GetGRFTownNameList`: valid until the next `NewGRF` load.
///
/// # Safety
/// As [`get_grf_town_name`]; the list is used only until the next `NewGRF` load.
#[allow(unsafe_code, reason = "the process-wide list")]
pub(crate) unsafe fn get_grf_town_name_list() -> &'static [StringID] {
    // SAFETY: The caller's contract.
    unsafe { &*GRF_TOWNNAME_NAMES.get() }
}

/// `CleanUpGRFTownNames`.
///
/// # Safety
/// As [`get_grf_town_name`].
#[allow(unsafe_code, reason = "the process-wide list")]
pub(crate) unsafe fn clean_up_grf_town_names() {
    unsafe { (*GRF_TOWNNAMES.get()).clear() };
}

/// `GetGRFTownNameId`.
///
/// # Safety
/// As [`get_grf_town_name`].
#[allow(unsafe_code, reason = "the process-wide list")]
#[allow(
    clippy::cast_possible_truncation,
    reason = "static_cast<uint16_t>(t.styles.size()) (newgrf_townname.cpp)"
)]
pub(crate) unsafe fn get_grf_town_name_id(mut gen_: u16) -> u32 {
    for t in unsafe { &*GRF_TOWNNAMES.get() } {
        if usize::from(gen_) < t.styles.len() {
            return t.grfid;
        }
        gen_ = gen_.wrapping_sub(t.styles.len() as u16);
    }
    // Fallback to no NewGRF
    0
}

/// `GetGRFTownNameType`.
///
/// # Safety
/// As [`get_grf_town_name`].
#[allow(unsafe_code, reason = "the process-wide list")]
#[allow(
    clippy::cast_possible_truncation,
    reason = "static_cast<uint16_t>(t.styles.size()) and the uint16_t return of SPECSTR_TOWNNAME_START (newgrf_townname.cpp)"
)]
pub(crate) unsafe fn get_grf_town_name_type(mut gen_: u16) -> u16 {
    for t in unsafe { &*GRF_TOWNNAMES.get() } {
        if usize::from(gen_) < t.styles.len() {
            return gen_;
        }
        gen_ = gen_.wrapping_sub(t.styles.len() as u16);
    }
    // Fallback to the first built in town name (English).
    SPECSTR_TOWNNAME_START.0 as u16
}

/// Action 0x0F - Define Town names (`FeatureTownName`).
///
/// # Safety
/// Runs inside the loader's entry for the current file, which is live, holding the
/// game-state mutex.
#[allow(
    unsafe_code,
    reason = "the loader's current file and the process-wide list"
)]
#[allow(
    clippy::too_many_lines,
    reason = "one original function, FeatureTownName"
)]
#[allow(
    clippy::cast_possible_truncation,
    reason = "GB(part.prob, 0, 7) added to the uint16_t maxprob (newgrf/newgrf_actf.cpp)"
)]
pub(crate) unsafe fn feature_town_name(
    s: &Services,
    buf: &mut ByteReader,
) -> Result<(), ReadBounds> {
    /* <0F> <id> <style-name> <num-parts> <parts>
     *
     * B id          ID of this definition in bottom 7 bits (final definition if bit 7 set)
     * V style-name  Name of the style (only for final definition)
     * B num-parts   Number of parts in this definition
     * V parts       The parts */
    unsafe {
        let grffile = GRFFileRef::of(s.shared(), newgrf_loading::file().cast());
        let grfid = grffile.grfid();

        // Only `DelGRFTownName` changes the list before this entry returns.
        let townname = add_grf_town_name(grfid);

        let mut id = buf.byte()?;
        grf_msg!(6, "FeatureTownName: definition 0x{:02X}", id & 0x7F);

        if has_bit(id, 7) {
            // Final definition
            id &= 0x7F;
            let new_scheme = grffile.grf_version() >= 7;

            let mut lang = buf.byte()?;
            let mut style;

            loop {
                lang &= 0x7F;

                let name = buf.string();
                let name = crate::utf8::borrow(name.0, name.1);

                let lang_name = crate::newgrf_text::translate(
                    &*s.text,
                    grfid,
                    lang,
                    false,
                    name,
                    SCC_NEWGRF_PRINT_WORD_STRING_ID,
                );
                grf_msg!(
                    6,
                    "FeatureTownName: lang 0x{:X} -> '{}'",
                    lang,
                    Bytes(&lang_name)
                );

                style = add_grf_string(
                    &*s.text,
                    &AddGRFString {
                        grfid,
                        stringid: GRFStringID(u32::from(id)),
                        langid_to_add: lang,
                        new_scheme,
                        allow_newlines: false,
                        text_to_add: name,
                        def_string: STR_UNDEFINED,
                    },
                );

                lang = buf.byte()?;
                if lang == 0 {
                    break;
                }
            }
            (*townname).styles.push(TownNameStyle { name: style, id });
        }

        let parts = buf.byte()?;
        grf_msg!(6, "FeatureTownName: {} parts", parts);

        let lists = &raw mut (*townname).partlists[usize::from(id)];
        (*lists).reserve(usize::from(parts));
        for partnum in 0..u32::from(parts) {
            let index = (*lists).len();
            (*lists).push(NamePartList::default());
            let partlist = &raw mut (&mut *lists)[index];
            let texts = buf.byte()?;
            (*partlist).bitstart = buf.byte()?;
            (*partlist).bitcount = buf.byte()?;
            (*partlist).maxprob = 0;
            grf_msg!(
                6,
                "FeatureTownName: part {} contains {} texts and will use GB(seed, {}, {})",
                partnum,
                texts,
                (*partlist).bitstart,
                (*partlist).bitcount
            );

            (*partlist).parts.reserve(usize::from(texts));
            for textnum in 0..u32::from(texts) {
                let index = (*partlist).parts.len();
                (*partlist).parts.push(NamePart::default());
                let part = &raw mut (&mut (*partlist).parts)[index];
                (*part).prob = buf.byte()?;

                if has_bit((*part).prob, 7) {
                    let ref_id = buf.byte()?;
                    if usize::from(ref_id) >= GRFTownName::MAX_LISTS
                        || (*townname).partlists[usize::from(ref_id)].is_empty()
                    {
                        grf_msg!(
                            0,
                            "FeatureTownName: definition 0x{:02X} doesn't exist, deactivating",
                            ref_id
                        );
                        del_grf_town_name(grfid);
                        newgrf_loading::disable_invalid_id(s);
                        return Ok(());
                    }
                    (*part).id = ref_id;
                    grf_msg!(
                        6,
                        "FeatureTownName: part {}, text {}, uses intermediate definition 0x{:02X} (with probability {})",
                        partnum,
                        textnum,
                        ref_id,
                        (*part).prob & 0x7F
                    );
                } else {
                    let text = buf.string();
                    (*part).text = crate::newgrf_text::translate(
                        &*s.text,
                        grfid,
                        0,
                        false,
                        crate::utf8::borrow(text.0, text.1),
                        SCC_NEWGRF_PRINT_WORD_STRING_ID,
                    );
                    let text = &raw const (*part).text;
                    grf_msg!(
                        6,
                        "FeatureTownName: part {}, text {}, '{}' (with probability {})",
                        partnum,
                        textnum,
                        Bytes(&*text),
                        (*part).prob
                    );
                }
                let prob = gb(u32::from((*part).prob), 0, 7) as u16;
                (*partlist).maxprob = (*partlist).maxprob.wrapping_add(prob);
            }
            grf_msg!(
                6,
                "FeatureTownName: part {}, total probability {}",
                partnum,
                (*partlist).maxprob
            );
        }
    }
    Ok(())
}
