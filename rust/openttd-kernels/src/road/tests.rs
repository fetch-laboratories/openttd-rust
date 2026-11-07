/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Replaces the native movement-table comparison. The oracle is transcribed from
//! `OpenTTD` 15.3 (14ec60f248547d4d062a1160f0fc26d742319888), `table/roadveh_movement.h`.
//! Each line is tram, state, then every original (x, y) byte pair in frame order.
//! It covers the tram/articulated table gap without claiming controller coverage.

#[test]
fn original_movement_and_stop_tables() {
    for line in include_str!("tests/movement-oracle.txt").lines() {
        let mut columns = line.split_whitespace();
        let tram = columns.next().unwrap() == "1";
        let state = columns.next().unwrap().parse::<usize>().unwrap();
        let row = columns.next().unwrap();
        for frame in 0..row.len() / 4 {
            let start = frame * 4;
            let x = u8::from_str_radix(&row[start..start + 2], 16).unwrap();
            let y = u8::from_str_radix(&row[start + 2..start + 4], 16).unwrap();
            assert_eq!(
                crate::road_data::entry(tram, state, frame),
                (x, y),
                "tram={tram}, state={state}, frame={frame}"
            );
        }
    }
    let stop_frames = [
        20, 20, 16, 16, 20, 20, 16, 16, 19, 19, 15, 15, 19, 19, 15, 15, 16, 16, 12, 12, 16, 16, 12,
        12, 15, 15, 11, 11, 15, 15, 11, 11,
    ];
    for (index, expected) in (0_u32..).zip(stop_frames) {
        assert_eq!(super::openttd_rust_road_stop_frame(index), expected);
    }
}
