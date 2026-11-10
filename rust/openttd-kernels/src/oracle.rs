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

impl<'a> Section<'a> {
    /// The stored output of case `index`, for tests that need it before running the case.
    pub(crate) fn case(&self, index: usize) -> Option<&'a str> {
        self.cases
            .get(index)
            .map(|&output| self.outputs[output as usize])
    }
    /// The number of cases.
    pub(crate) fn len(&self) -> usize {
        self.cases.len()
    }
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
    check_with(sections, name, actual, |want, got| {
        let mut out = String::new();
        let (a, b): (Vec<_>, Vec<_>) = (want.split(' ').collect(), got.split(' ').collect());
        for field in 0..a.len().max(b.len()) {
            let (x, y) = (a.get(field).copied(), b.get(field).copied());
            if x != y {
                let _ = writeln!(
                    out,
                    "  field {field}: original {} candidate {}",
                    x.unwrap_or("<none>"),
                    y.unwrap_or("<none>")
                );
            }
        }
        out
    })
}

/// Compare rows of `u64` results stored with [`encode_rows`]; `(inputs, results, stride)`.
///
/// A failure decodes the original row and lists each differing element position.
pub(crate) fn check_rows(
    sections: &HashMap<&str, Section<'_>>,
    name: &str,
    actual: impl IntoIterator<Item = (String, Vec<u64>, usize)>,
) -> usize {
    let actual = actual
        .into_iter()
        .map(|(inputs, values, stride)| (inputs, encode_rows(&values, stride)));
    check_with(sections, name, actual, |want, got| {
        let mut out = String::new();
        let (a, b) = (decode_rows(want), decode_rows(got));
        for index in 0..a.len().max(b.len()) {
            let (x, y) = (a.get(index), b.get(index));
            if x != y {
                let _ = writeln!(out, "  element {index}: original {x:?} candidate {y:?}");
            }
        }
        out
    })
}

/// Encode a result row losslessly: `stride` interleaved parts joined by ` | `, each part its
/// first value in decimal, then wrapping `i64` deltas with runs written `d*n` (`-` if empty).
pub(crate) fn encode_rows(values: &[u64], stride: usize) -> String {
    let mut parts = Vec::with_capacity(stride);
    for start in 0..stride {
        let part: Vec<u64> = values.iter().skip(start).step_by(stride).copied().collect();
        let Some(&first) = part.first() else {
            parts.push("-".to_owned());
            continue;
        };
        let mut text = first.to_string();
        let deltas: Vec<i64> = part
            .windows(2)
            .map(|pair| pair[1].wrapping_sub(pair[0]).cast_signed())
            .collect();
        for run in deltas.chunk_by(|a, b| a == b) {
            if run.len() > 1 {
                let _ = write!(text, " {}*{}", run[0], run.len());
            } else {
                let _ = write!(text, " {}", run[0]);
            }
        }
        parts.push(text);
    }
    parts.join(" | ")
}

/// Decode an [`encode_rows`] line.
pub(crate) fn decode_rows(line: &str) -> Vec<u64> {
    let parts: Vec<Vec<u64>> = line
        .split(" | ")
        .map(|part| {
            let mut values: Vec<u64> = Vec::new();
            if part == "-" {
                return values;
            }
            let mut tokens = part.split(' ');
            values.push(
                tokens
                    .next()
                    .and_then(|t| t.parse().ok())
                    .unwrap_or_default(),
            );
            for token in tokens {
                let (delta, count) = token.split_once('*').unwrap_or((token, "1"));
                let delta: i64 = delta.parse().unwrap_or_default();
                for _ in 0..count.parse::<usize>().unwrap_or(1) {
                    let last = *values.last().unwrap_or(&0);
                    values.push(last.wrapping_add_signed(delta));
                }
            }
            values
        })
        .collect();
    let total = parts.iter().map(Vec::len).sum();
    (0..total)
        .map(|index| parts[index % parts.len()][index / parts.len()])
        .collect()
}

fn check_with(
    sections: &HashMap<&str, Section<'_>>,
    name: &str,
    actual: impl IntoIterator<Item = (String, String)>,
    differ: impl Fn(&str, &str) -> String,
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
                failures.push_str(&differ(want, &got));
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
