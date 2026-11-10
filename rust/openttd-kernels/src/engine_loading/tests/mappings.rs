/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Frozen observations of the unchanged 15.3 engine mapping manager. These calls use
//! the production registry, including a joined game thread, rather than a local map.
use super::*;
use std::fmt::Write;

fn mapping_snapshot(kind: u8) -> Vec<Mapping> {
    // SAFETY: Only this test uses MAPPINGS; registry mutation is serialized.
    let view = unsafe { openttd_rust_engine_mappings(kind) };
    // SAFETY: Copy the live view before any further registry mutation or thread.
    unsafe { std::slice::from_raw_parts(view.data, view.size) }.to_vec()
}

fn record_entry(observed: &mut String, operation: &str, entry: Mapping) {
    writeln!(
        observed,
        "{operation} {} {} {} {} {}",
        entry.grfid, entry.internal_id, entry.kind, entry.substitute_id, entry.engine
    )
    .unwrap();
}

#[test]
fn reservation_replacement_and_serialized_threads_match_original() {
    // Original engine.cpp ResetToDefaultMapping/GetID/UseUnreservedID/SetID,
    // baseline 14ec60f248547d4d062a1160f0fc26d742319888, GNU/Linux x86_64.
    // Counts are the original table sizes. The trace includes every assertion
    // from the retired native mapping case, including unchanged substitute_id.
    let mut observed = String::new();
    // SAFETY: This is the only test using MAPPINGS. Joined threads serialize all calls.
    unsafe { openttd_rust_engine_mapping_defaults([116, 88, 11, 41].as_ptr()) };
    for kind in 0..VEH_COMPANY_END {
        // SAFETY: Every lookup runs before the next serialized registry mutation.
        let get = |local, grf| unsafe { openttd_rust_engine_mapping_get(kind, local, grf) };
        // SAFETY: Only this thread mutates MAPPINGS during reservation.
        let reserve = |static_access| unsafe {
            use_unreserved_id(crate::vehicle_type::VehicleType(kind), 1, 17, static_access).0
        };
        let size = mapping_snapshot(kind).len();
        let first = get(0, INVALID_GRFID);
        writeln!(observed, "default {kind} {size} {first}").unwrap();
        writeln!(observed, "static {} {}", reserve(true), get(1, 17)).unwrap();
        let reserved = reserve(false);
        let unreserved = get(1, INVALID_GRFID);
        writeln!(observed, "reserve {reserved} {unreserved} {}", get(1, 17)).unwrap();
        let visible = std::thread::spawn(move || {
            // SAFETY: The parent does not access MAPPINGS until it joins this thread.
            let value = unsafe { openttd_rust_engine_mapping_get(kind, 1, 17) };
            // SAFETY: Lookup is finished; this thread alone mutates the registry.
            unsafe { openttd_rust_engine_mapping_set(kind, 1, 17, 99, 500) };
            value
        })
        .join()
        .unwrap();
        writeln!(observed, "thread {visible}").unwrap();
        record_entry(&mut observed, "replace", mapping_snapshot(kind)[0]);
        // SAFETY: The game thread has joined and no view into the registry remains.
        unsafe { openttd_rust_engine_mapping_set(kind, 300, 12, 44, 501) };
        let map = mapping_snapshot(kind);
        let sorted = map.windows(2).all(|pair| pair[0].key() <= pair[1].key());
        let mut entry = String::new();
        record_entry(&mut entry, "insert", map[0]);
        writeln!(observed, "{} {}", entry.trim_end(), u8::from(sorted)).unwrap();
    }
    // SAFETY: All worker threads have joined and registry views are owned snapshots.
    unsafe { openttd_rust_engine_mapping_clear() };
    assert_eq!(
        observed,
        include_str!("mapping-oracle.txt").replace("\r\n", "\n")
    );
}
