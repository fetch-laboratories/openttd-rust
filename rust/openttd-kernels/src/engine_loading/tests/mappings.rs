/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Frozen observations of the unchanged 15.3 engine mapping manager. These calls use
//! the production registry, including a joined game thread, rather than a local map.
use super::*;
use std::fmt::Write;

#[test]
fn reservation_replacement_and_serialized_threads_match_original() {
    // Original engine.cpp ResetToDefaultMapping/GetID/UseUnreservedID/SetID,
    // baseline 14ec60f248547d4d062a1160f0fc26d742319888, GNU/Linux x86_64.
    // Counts are the original table sizes. The trace includes every assertion
    // from the retired native mapping case, including unchanged substitute_id.
    let mut observed = String::new();
    // SAFETY: This is the only test using MAPPINGS. Each spawned thread is joined
    // before the caller accesses the registry again, matching the game mutex.
    unsafe {
        openttd_rust_engine_mapping_defaults([116, 88, 11, 41].as_ptr());
        for kind in 0..VEH_COMPANY_END {
            let view = openttd_rust_engine_mappings(kind);
            let get = |local, grf| openttd_rust_engine_mapping_get(kind, local, grf);
            writeln!(
                observed,
                "default {kind} {} {}",
                view.size,
                get(0, INVALID_GRFID)
            )
            .unwrap();
            writeln!(
                observed,
                "static {} {}",
                openttd_rust_engine_mapping_use(kind, 1, 17, true),
                get(1, 17)
            )
            .unwrap();
            writeln!(
                observed,
                "reserve {} {} {}",
                openttd_rust_engine_mapping_use(kind, 1, 17, false),
                get(1, INVALID_GRFID),
                get(1, 17)
            )
            .unwrap();
            let visible = std::thread::spawn(move || {
                let value = openttd_rust_engine_mapping_get(kind, 1, 17);
                openttd_rust_engine_mapping_set(kind, 1, 17, 99, 500);
                value
            })
            .join()
            .unwrap();
            writeln!(observed, "thread {visible}").unwrap();
            let view = openttd_rust_engine_mappings(kind);
            let entry = *view.data;
            writeln!(
                observed,
                "replace {} {} {} {} {}",
                entry.grfid, entry.internal_id, entry.kind, entry.substitute_id, entry.engine
            )
            .unwrap();
            openttd_rust_engine_mapping_set(kind, 300, 12, 44, 501);
            let view = openttd_rust_engine_mappings(kind);
            let map = std::slice::from_raw_parts(view.data, view.size);
            let entry = map[0];
            let sorted = map.windows(2).all(|pair| pair[0].key() <= pair[1].key());
            writeln!(
                observed,
                "insert {} {} {} {} {} {}",
                entry.grfid,
                entry.internal_id,
                entry.kind,
                entry.substitute_id,
                entry.engine,
                u8::from(sorted)
            )
            .unwrap();
        }
        openttd_rust_engine_mapping_clear();
    }
    assert_eq!(observed, include_str!("mapping-oracle.txt"));
}
