/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Test-only reader for frozen pinned-original observation streams (#736 oracle rule).
//!
//! An oracle file holds `#` provenance lines, then one or more sections:
//!
//! ```text
//! @@ section-name
//! distinct output 0
//! distinct output 1
//! %%
//! 0 1*3 0
//! ```
//!
//! Each distinct output line appears once, in first-appearance order. After `%%`, the
//! whitespace-separated case references name one distinct output per enumerated case, in
//! case order; `i*n` repeats output `i` for `n` consecutive cases. Inputs are never stored:
//! the test regenerates the documented input domain in the same order, and a failure prints
//! that case's inputs with the differing space-separated fields.

use std::collections::HashMap;
use std::fmt::Write as _;

/// One frozen observation stream.
pub(crate) struct Section<'a> {
    outputs: Vec<&'a str>,
    cases: Vec<u32>,
}

/// Parse every section of an oracle file.
pub(crate) fn parse(text: &str) -> HashMap<&str, Section<'_>> {
    let mut sections = HashMap::new();
    let mut lines = text
        .lines()
        .filter(|line| !line.starts_with('#'))
        .peekable();
    while let Some(header) = lines.next() {
        let name = header
            .strip_prefix("@@ ")
            .unwrap_or_else(|| panic!("oracle section header expected, found {header:?}"));
        let mut outputs = Vec::new();
        for line in lines.by_ref() {
            if line == "%%" {
                break;
            }
            outputs.push(line);
        }
        let mut cases = Vec::new();
        while let Some(line) = lines.next_if(|line| !line.starts_with("@@ ")) {
            for token in line.split_whitespace() {
                let (index, count) = token.split_once('*').unwrap_or((token, "1"));
                let index: u32 = index.parse().expect("oracle case reference");
                assert!(
                    (index as usize) < outputs.len(),
                    "oracle reference out of range"
                );
                let count: usize = count.parse().expect("oracle repeat count");
                cases.extend(std::iter::repeat_n(index, count));
            }
        }
        assert!(
            sections.insert(name, Section { outputs, cases }).is_none(),
            "duplicate oracle section {name}"
        );
    }
    sections
}

/// Compare a section with the regenerated cases: `(inputs, actual output)` in case order.
///
/// Panics listing up to 20 differing cases, each with its inputs and differing fields, and
/// on any count mismatch. Returns the number of compared cases.
pub(crate) fn check(
    sections: &HashMap<&str, Section<'_>>,
    name: &str,
    actual: impl IntoIterator<Item = (String, String)>,
) -> usize {
    let section = sections
        .get(name)
        .unwrap_or_else(|| panic!("missing oracle section {name}"));
    let mut failures = String::new();
    let mut failed = 0_usize;
    let mut count = 0_usize;
    for (inputs, got) in actual {
        let Some(&index) = section.cases.get(count) else {
            count += 1;
            continue;
        };
        count += 1;
        let want = section.outputs[index as usize];
        if want != got {
            failed += 1;
            if failed <= 20 {
                let _ = writeln!(failures, "case {} [{inputs}]:", count - 1);
                let (a, b): (Vec<_>, Vec<_>) =
                    (want.split(' ').collect(), got.split(' ').collect());
                for field in 0..a.len().max(b.len()) {
                    let (x, y) = (a.get(field).copied(), b.get(field).copied());
                    if x != y {
                        let _ = writeln!(
                            failures,
                            "  field {field}: original {} candidate {}",
                            x.unwrap_or("<none>"),
                            y.unwrap_or("<none>")
                        );
                    }
                }
            }
        }
    }
    assert!(
        failed == 0 && count == section.cases.len(),
        "{name}: {failed} differing cases; {count} regenerated cases, {} original cases\n{failures}",
        section.cases.len()
    );
    count
}
