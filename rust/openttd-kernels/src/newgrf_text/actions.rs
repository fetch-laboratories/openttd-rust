/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! GRF text loading actions. Checked reads return to the native loader error frame;
//! earlier string publications remain visible if a later read fails.
use crate::debug::{Bytes, grf_msg};
use crate::newgrf_bytereader::{Cursor as ByteReader, ReadBounds};
use crate::newgrf_config::{GCS_ACTIVATED, GCS_INITIALISED};
use crate::newgrf_loading::{self as loading, Services};
use crate::string_validation::make_valid;
use std::ffi::c_void;

pub(crate) unsafe fn translate_grf_strings(
    s: &Services,
    buf: &mut ByteReader,
) -> Result<(), ReadBounds> {
    unsafe {
        let grfid = buf.dword()?;
        let config = (s.find_config)(grfid, u32::MAX);
        let status = if config.is_null() {
            None
        } else {
            Some((s.config_status)(config))
        };
        let text = &*s.text;
        if !matches!(status, Some(GCS_INITIALISED | GCS_ACTIVATED)) {
            grf_msg!(
                7,
                "TranslateGRFStrings: GRFID 0x{:08X} unknown, skipping action 13",
                grfid.swap_bytes()
            );
            return Ok(());
        }
        if status == Some(GCS_INITIALISED) {
            let error = loading::openttd_rust_grf_disable(s, s.load_after, std::ptr::null_mut());
            (s.error_after_translated_file)(error);
            return Ok(());
        }
        let language = if loading::file_version(s, loading::file()) >= 8 {
            buf.byte()?
        } else {
            0x7f
        };
        let num_strings = buf.byte()?;
        let first_id = u32::from(buf.word()?);
        let end = first_id + u32::from(num_strings);
        if !((first_id >= 0xd000 && end <= 0xd400) || (first_id >= 0xd800 && end <= 0xe000)) {
            grf_msg!(
                7,
                "TranslateGRFStrings: Attempting to set out-of-range string IDs in action 13 (first: 0x{first_id:04X}, number: 0x{num_strings:02X})"
            );
            return Ok(());
        }
        for i in 0..num_strings {
            if buf.remaining() == 0 {
                break;
            }
            let string = buf.string();
            if string.1 == 0 {
                grf_msg!(7, "TranslateGRFString: Ignoring empty string.");
                continue;
            }
            super::openttd_rust_grf_text_add(
                text,
                grfid,
                first_id + u32::from(i),
                language,
                true,
                true,
                string.0,
                string.1,
                s.undefined,
            );
        }
        Ok(())
    }
}

#[repr(C)]
pub struct BindingServices {
    pub(crate) badge_bind: unsafe extern "C" fn(*mut c_void, u16, u8, *const c_void) -> bool,
    pub(crate) badge_name: unsafe extern "C" fn(*mut c_void, u16) -> *mut u32,
    pub(crate) canal_bind: unsafe extern "C" fn(u8, *const c_void, *const c_void),
    pub(crate) cargo_bind: unsafe extern "C" fn(u8, *const c_void, *const c_void),
    pub(crate) mark_feature: unsafe extern "C" fn(*mut c_void, u8),
    pub(crate) object_spec: unsafe extern "C" fn(*mut c_void, u16) -> *mut crate::objects::Spec,
    pub(crate) rail_bind: unsafe extern "C" fn(u8, u8, *const c_void, *const c_void),
    pub(crate) road_bind: unsafe extern "C" fn(u8, u8, *const c_void, *const c_void),
}
crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDGrfBindingServices" ["grf_loading"] => BindingServices { badge_bind, badge_name, canal_bind, cargo_bind, mark_feature, object_spec, rail_bind, road_bind };
}

pub(crate) unsafe fn feature_new_name(
    s: &Services,
    buf: &mut ByteReader,
) -> Result<(), ReadBounds> {
    use crate::newgrf::{GSF_AIRCRAFT, GSF_BADGES, GSF_END};
    unsafe {
        let file = loading::file();
        let new_scheme = loading::file_version(s, file) >= 7;
        let feature = buf.byte()?;
        let text = &*s.text;
        if feature >= GSF_END && feature != crate::newgrf::GSF_ORIGINAL_STRINGS {
            grf_msg!(
                1,
                "FeatureNewName: Unsupported feature 0x{feature:02X}, skipping"
            );
            return Ok(());
        }
        let mut language = buf.byte()?;
        let count = buf.byte()?;
        let generic = language & 0x80 != 0;
        let mut id = if generic {
            buf.word()?
        } else if feature <= GSF_AIRCRAFT || feature == GSF_BADGES {
            buf.extended()?
        } else {
            u16::from(buf.byte()?)
        };
        language &= 0x7f;
        let end = id.wrapping_add(u16::from(count));
        grf_msg!(
            6,
            "FeatureNewName: About to rename engines {id}..{end} (feature 0x{feature:02X}) in language 0x{language:02X}"
        );
        let overlay = if generic {
            0
        } else {
            (u32::from(feature) + 1) << 16
        };
        let bindings = &*s.bindings;
        let grfid = (s.shared().grf_file_id)(file);
        while id < end && buf.remaining() != 0 {
            let name = buf.string();
            grf_msg!(
                8,
                "FeatureNewName: 0x{:04X} <- {}",
                id,
                Bytes(&make_valid(
                    std::slice::from_raw_parts(name.0, name.1),
                    crate::string_type::StringValidationSettings::from_element(
                        crate::string_type::StringValidationSetting::ReplaceWithQuestionMark
                    )
                ))
            );
            if feature <= GSF_AIRCRAFT {
                if generic {
                    super::openttd_rust_grf_text_add(
                        text,
                        grfid,
                        u32::from(id),
                        language,
                        new_scheme,
                        true,
                        name.0,
                        name.1,
                        s.undefined,
                    );
                } else {
                    let engine = crate::engine_loading::openttd_rust_engine_definition(
                        &*s.engine,
                        file,
                        feature,
                        id,
                        loading::is_static(s, loading::config()),
                    );
                    if !engine.is_null() {
                        let index = ((*(*s.engine).resolver).capabilities.id)(engine);
                        let default = (*engine).info.string_id;
                        let translated = super::openttd_rust_grf_text_add(
                            text,
                            grfid,
                            overlay | u32::from(index),
                            language,
                            new_scheme,
                            false,
                            name.0,
                            name.1,
                            default,
                        );
                        (*engine.cast_mut()).info.string_id = translated;
                    }
                }
            } else if feature == GSF_BADGES {
                if generic {
                    super::openttd_rust_grf_text_add(
                        text,
                        grfid,
                        u32::from(id),
                        language,
                        new_scheme,
                        true,
                        name.0,
                        name.1,
                        s.undefined,
                    );
                } else {
                    let string = (bindings.badge_name)(file, id);
                    if string.is_null() {
                        grf_msg!(
                            1,
                            "FeatureNewName: Attempt to name undefined badge 0x{id:X}, ignoring"
                        );
                    } else {
                        let translated = super::openttd_rust_grf_text_add(
                            text,
                            grfid,
                            overlay | u32::from(id),
                            language,
                            true,
                            false,
                            name.0,
                            name.1,
                            s.undefined,
                        );
                        string.write(translated);
                    }
                }
            } else if (0xd000..0xd400).contains(&id) || id >= 0xd800 {
                super::openttd_rust_grf_text_add(
                    text,
                    grfid,
                    u32::from(id),
                    language,
                    new_scheme,
                    true,
                    name.0,
                    name.1,
                    s.undefined,
                );
            } else {
                let local = u32::from(id & 0xff);
                let mut class_spec = std::ptr::null_mut();
                let specs = s.shared().specs();
                let string = match id >> 8 {
                    0xc4 | 0xc5 => {
                        let spec = ((*s.stations).station_get)(file, local);
                        if spec.is_null() {
                            grf_msg!(
                                1,
                                "FeatureNewName: Attempt to name undefined station 0x{local:X}, ignoring"
                            );
                            std::ptr::null_mut()
                        } else if id >> 8 == 0xc4 {
                            class_spec = spec;
                            std::ptr::null_mut()
                        } else {
                            (specs.station().name)(spec)
                        }
                    }
                    0xc7 => {
                        let owner = &*s.airports;
                        let spec = if local as usize >= (owner.tile_count)(file) {
                            std::ptr::null_mut()
                        } else {
                            (owner.tile_get)(file, local as usize)
                        };
                        if spec.is_null() {
                            grf_msg!(
                                1,
                                "FeatureNewName: Attempt to name undefined airport tile 0x{local:X}, ignoring"
                            );
                            std::ptr::null_mut()
                        } else {
                            (specs.airport_tile().name)(spec)
                        }
                    }
                    0xc9 => {
                        let owner = &*s.houses;
                        let spec = if local as usize >= (owner.local_count)(file) {
                            std::ptr::null_mut()
                        } else {
                            (owner.local)(file, local as usize)
                                .cast_mut()
                                .cast::<crate::house::HouseSpec>()
                        };
                        if spec.is_null() {
                            grf_msg!(
                                1,
                                "FeatureNewName: Attempt to name undefined house 0x{local:X}, ignoring."
                            );
                            std::ptr::null_mut()
                        } else {
                            (specs.house().name)(spec)
                        }
                    }
                    _ => {
                        grf_msg!(7, "FeatureNewName: Unsupported ID (0x{id:04X})");
                        std::ptr::null_mut()
                    }
                };
                if !class_spec.is_null() || !string.is_null() {
                    let translated = super::openttd_rust_grf_text_add(
                        text,
                        grfid,
                        u32::from(id),
                        language,
                        new_scheme,
                        false,
                        name.0,
                        name.1,
                        s.undefined,
                    );
                    if class_spec.is_null() {
                        string.write(translated);
                    } else {
                        crate::newgrf_class::station::openttd_rust_station_class_set_name(
                            class_spec, translated,
                        );
                    }
                }
            }
            id = id.wrapping_add(1);
        }
        Ok(())
    }
}
