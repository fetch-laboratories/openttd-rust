/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! `newgrf_townname.cpp` and Action 0F (`newgrf_actf.cpp`): `NewGRF` town-name storage,
//! parsing and generation. Both lists are process-wide; every access holds the game-state
//! mutex. Borrows of the lists end before each native string or log service, none of which
//! reenters this module.
#![allow(
    unsafe_code,
    clippy::missing_safety_doc,
    clippy::cast_possible_truncation
)]
use crate::bitmath_func::{gb, has_bit};
use crate::game_state::GameState;
use crate::newgrf_bytereader::{Cursor as ByteReader, ReadBounds};
use crate::newgrf_loading::{self, Services};
use crate::services::ByteSpan;
use crate::strings_type::SPECSTR_TOWNNAME_START;
use std::ffi::c_void;

/// `GRFTownName::MAX_LISTS`.
const MAX_LISTS: usize = 128;

#[derive(Default)]
struct NamePart {
    /// If probability bit 7 is clear.
    text: Vec<u8>,
    /// If probability bit 7 is set.
    id: u8,
    prob: u8,
}
#[derive(Default)]
struct NamePartList {
    bitstart: u8,
    bitcount: u8,
    maxprob: u16,
    parts: Vec<NamePart>,
}
struct TownNameStyle {
    name: u32,
    id: u8,
}
struct GrfTownName {
    grfid: u32,
    styles: Vec<TownNameStyle>,
    partlists: [Vec<NamePartList>; MAX_LISTS],
}

static TOWNNAMES: GameState<Vec<GrfTownName>> = GameState::new(Vec::new());
static NAMES: GameState<Vec<u32>> = GameState::new(Vec::new());

unsafe fn townnames<'a>() -> &'a mut Vec<GrfTownName> {
    unsafe { &mut *TOWNNAMES.get() }
}
unsafe fn get(grfid: u32) -> Option<&'static GrfTownName> {
    unsafe { townnames().iter().find(|t| t.grfid == grfid) }
}
/// `AddGRFTownName`, returning the entry's index.
unsafe fn add(grfid: u32) -> usize {
    let list = unsafe { townnames() };
    if let Some(index) = list.iter().position(|t| t.grfid == grfid) {
        return index;
    }
    list.push(GrfTownName {
        grfid,
        styles: Vec::new(),
        partlists: [const { Vec::new() }; MAX_LISTS],
    });
    list.len() - 1
}
/// `DelGRFTownName`; the entry was just added by the caller, so `find` succeeds.
unsafe fn del(grfid: u32) {
    let list = unsafe { townnames() };
    let index = list.iter().position(|t| t.grfid == grfid);
    list.remove(index.expect("DelGRFTownName erases end()"));
}

fn random_part(builder: &mut dyn FnMut(&[u8]), t: &GrfTownName, seed: u32, id: u8) {
    for partlist in &t.partlists[usize::from(id)] {
        let count = partlist.bitcount;
        let mut maxprob = partlist.maxprob;
        let r = gb(seed, partlist.bitstart, count)
            .wrapping_mul(u32::from(maxprob))
            .wrapping_shr(u32::from(count));
        for part in &partlist.parts {
            maxprob = maxprob.wrapping_sub(gb(u32::from(part.prob), 0, 7) as u16);
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

/// `GRFTownNameGenerate`: appends each selected part to the native builder through `append`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_townname_generate(
    shared: &crate::services::Services,
    builder: *mut c_void,
    append: unsafe extern "C" fn(*mut c_void, ByteSpan),
    grfid: u32,
    gen_: u16,
    seed: u32,
) {
    unsafe {
        generate(
            shared,
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

/// Append selected parts without crossing the native facade.
pub(crate) unsafe fn generate(
    shared: &crate::services::Services,
    builder: &mut dyn FnMut(&[u8]),
    grfid: u32,
    gen_: u16,
    seed: u32,
) {
    if let Some(t) = unsafe { get(grfid) } {
        crate::movement::native_assert(shared, usize::from(gen_) < t.styles.len());
        // C++ operator[] is unchecked when its native assertion is disabled.
        let id = unsafe { (*t.styles.as_ptr().add(usize::from(gen_))).id };
        random_part(builder, t, seed, id);
    }
}
/// `GetGRFTownName(grfid) != nullptr`.
pub(crate) unsafe fn exists(grfid: u32) -> bool {
    unsafe { get(grfid).is_some() }
}
/// `InitGRFTownGeneratorNames`.
pub(crate) unsafe fn init_generator_names() {
    let names = unsafe { &mut *NAMES.get() };
    names.clear();
    for t in unsafe { townnames() }.iter() {
        for style in &t.styles {
            names.push(style.name);
        }
    }
}
/// `GetGRFTownNameList`: valid until the next `NewGRF` load.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_grf_townname_list(size: *mut usize) -> *const u32 {
    let names = unsafe { &*NAMES.get() };
    unsafe { size.write(names.len()) };
    names.as_ptr()
}
/// `CleanUpGRFTownNames`.
pub(crate) unsafe fn clean_up() {
    unsafe { townnames().clear() };
}
/// `GetGRFTownNameId`.
pub(crate) unsafe fn get_id(mut gen_: u16) -> u32 {
    for t in unsafe { townnames() }.iter() {
        if usize::from(gen_) < t.styles.len() {
            return t.grfid;
        }
        gen_ = gen_.wrapping_sub(t.styles.len() as u16);
    }
    // Fallback to no NewGRF.
    0
}
/// `GetGRFTownNameType`.
pub(crate) unsafe fn get_type(mut gen_: u16) -> u16 {
    for t in unsafe { townnames() }.iter() {
        if usize::from(gen_) < t.styles.len() {
            return gen_;
        }
        gen_ = gen_.wrapping_sub(t.styles.len() as u16);
    }
    // Fallback to the first built in town name (English).
    SPECSTR_TOWNNAME_START as u16
}
unsafe fn translate(s: &Services, grfid: u32, lang: u8, text: (*const u8, usize)) -> Vec<u8> {
    // The native loader frame keeps this view alive until the next translation.
    let translated = unsafe { (s.translate_text)(grfid, lang, false, text.0, text.1) };
    if translated.size == 0 {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(translated.data, translated.size) }.to_vec()
    }
}

/// Action 0x0F - Define Town names (`FeatureTownName`).
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
        let file = newgrf_loading::file();
        let grfid = (s.file_id)(file);

        // Only `DelGRFTownName` changes the list before this entry returns.
        let index = add(grfid);
        let townname: *mut GrfTownName = &raw mut townnames()[index];

        let mut id = buf.byte()?;
        (s.town_definition)(id & 0x7F);

        if has_bit(id, 7) {
            // Final definition.
            id &= 0x7F;
            let new_scheme = newgrf_loading::file_version(s, file) >= 7;

            let mut lang = buf.byte()?;
            let mut style;
            loop {
                lang &= 0x7F;

                let name = buf.string();

                let lang_name = translate(s, grfid, lang, name);
                (s.town_lang)(lang, lang_name.as_ptr(), lang_name.len());

                style = (s.add_string)(
                    grfid,
                    u32::from(id),
                    lang,
                    new_scheme,
                    false,
                    name.0,
                    name.1,
                    s.undefined,
                );

                lang = buf.byte()?;
                if lang == 0 {
                    break;
                }
            }
            (*townname).styles.push(TownNameStyle { name: style, id });
        }

        let parts = buf.byte()?;
        (s.town_parts)(parts);

        let lists = &raw mut (*townname).partlists[usize::from(id)];
        (*lists).reserve(usize::from(parts));
        for partnum in 0..u32::from(parts) {
            (*lists).push(NamePartList::default());
            let partlist: *mut NamePartList = (*lists).last_mut().expect("emplace_back");
            let texts = buf.byte()?;
            (*partlist).bitstart = buf.byte()?;
            (*partlist).bitcount = buf.byte()?;
            (*partlist).maxprob = 0;
            (s.town_part)(partnum, texts, (*partlist).bitstart, (*partlist).bitcount);

            (*partlist).parts.reserve(usize::from(texts));
            for textnum in 0..u32::from(texts) {
                (*partlist).parts.push(NamePart::default());
                let part: *mut NamePart = (*partlist).parts.last_mut().expect("emplace_back");
                (*part).prob = buf.byte()?;

                if has_bit((*part).prob, 7) {
                    let ref_id = buf.byte()?;
                    if usize::from(ref_id) >= MAX_LISTS
                        || (*townname).partlists[usize::from(ref_id)].is_empty()
                    {
                        (s.town_missing)(ref_id);
                        del(grfid);
                        newgrf_loading::disable_invalid_id(s);
                        return Ok(());
                    }
                    (*part).id = ref_id;
                    (s.town_reference)(partnum, textnum, ref_id, (*part).prob & 0x7F);
                } else {
                    let text = buf.string();
                    (*part).text = translate(s, grfid, 0, text);
                    let text = &raw const (*part).text;
                    (s.town_text)(
                        partnum,
                        textnum,
                        (*text).as_ptr(),
                        (*text).len(),
                        (*part).prob,
                    );
                }
                let prob = gb(u32::from((*part).prob), 0, 7) as u16;
                (*partlist).maxprob = (*partlist).maxprob.wrapping_add(prob);
            }
            (s.town_total)(partnum, (*partlist).maxprob);
        }
    }
    Ok(())
}
