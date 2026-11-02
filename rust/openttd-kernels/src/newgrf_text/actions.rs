/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! GRF text loading actions. Checked reads return to the native loader error frame;
//! earlier string publications remain visible if a later read fails.
use crate::newgrf_bytereader::{Cursor as ByteReader, ReadBounds};
use crate::newgrf_config::{GCS_ACTIVATED, GCS_INITIALISED};
use crate::newgrf_loading::{self as loading, Services};

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
            text.log(
                7,
                format!(
                    "TranslateGRFStrings: GRFID 0x{:08X} unknown, skipping action 13",
                    grfid.swap_bytes()
                )
                .as_bytes(),
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
            text.log(7, format!("TranslateGRFStrings: Attempting to set out-of-range string IDs in action 13 (first: 0x{first_id:04X}, number: 0x{num_strings:02X})").as_bytes());
            return Ok(());
        }
        for i in 0..num_strings {
            if buf.remaining() == 0 {
                break;
            }
            let string = buf.string();
            if string.1 == 0 {
                text.log(7, b"TranslateGRFString: Ignoring empty string.");
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
