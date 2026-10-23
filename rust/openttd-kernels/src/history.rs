/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Staged generic history structure; all typed operations remain in C++.

/// By-value descriptor fields; tokens identify C++ objects but are never dereferenced here.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Descriptor {
    /// Child descriptor identity, zero when absent.
    pub child: usize,
    /// Number of reportable periods.
    pub periods: u8,
    /// Number of stored records.
    pub records: u8,
    /// First stored index.
    pub first: u8,
    /// Exclusive last stored index.
    pub last: u8,
    /// Number of child records in one period.
    pub division: u8,
    /// Number of initial periods in one period.
    pub total_division: u8,
    /// Child period count, zero when absent.
    pub child_periods: u8,
    /// Child division, zero when absent.
    pub child_division: u8,
}

/// One structural instruction; no typed data or callback crosses the ABI.
#[repr(C)]
#[derive(Default)]
pub struct Step {
    /// 0 done, 1 describe, 2 shift, 3 copy, 4 reset, 5 rotate reduction,
    /// 6 construct scratch, 7 child query, 8 query reduction, 9 leaf, 10 fatal.
    pub kind: u8,
    /// Number of typed inputs for a reduction or scratch phase.
    pub count: u8,
    /// Source index or shift beginning.
    pub first: u32,
    /// Exclusive shift end.
    pub last: u32,
    /// Destination index in history or the current scratch array.
    pub target: u32,
    /// Descriptor identity for a describe instruction only.
    pub token: usize,
    /// Final mask for update or boolean validity otherwise.
    pub value: u64,
}

#[derive(Clone, Copy)]
enum Stage {
    Enter,
    AfterChild,
    Leading,
    Reset,
    Pop,
    AwaitPhase,
    Children,
    Complete,
}

struct Frame {
    token: usize,
    descriptor: Option<Descriptor>,
    age: u32,
    stage: Stage,
    start: u32,
    child_index: u32,
    valid: bool,
}

impl Frame {
    fn new(token: usize, age: u32) -> Self {
        Self {
            token,
            descriptor: None,
            age,
            stage: Stage::Enter,
            start: 0,
            child_index: 0,
            valid: false,
        }
    }
}

fn has_bit(mask: u64, slot: u32) -> bool {
    // HasBit's C++ uint8 parameter narrows the unsigned index before shifting.
    let slot = slot.to_le_bytes()[0];
    mask & (1_u64 << slot) != 0
}

fn aggregate(d: Descriptor, age: u32) -> bool {
    // C++ first subtracts promoted signed ints, then converts to native uint.
    let threshold = (i32::from(d.child_periods) - i32::from(d.division)).to_ne_bytes();
    d.child != 0 && age.wrapping_mul(u32::from(d.division)) < u32::from_ne_bytes(threshold)
}

fn start(d: Descriptor, age: u32, month: u32) -> u32 {
    age.wrapping_mul(u32::from(d.division))
        .wrapping_add((month / u32::from(d.child_division)) % u32::from(d.division))
}

fn slot(d: Descriptor, age: u32) -> u32 {
    let offset = if d.child == 0 {
        0
    } else {
        (u32::from(d.child_periods) / u32::from(d.division)).wrapping_sub(1)
    };
    u32::from(d.first).wrapping_add(age).wrapping_sub(offset)
}

pub(crate) fn update(mask: u64, d: Descriptor, month: u32) -> u64 {
    if has_bit(mask, u32::from(d.last).wrapping_sub(1))
        || !month.is_multiple_of(u32::from(d.total_division))
        || (d.division != 1
            && !has_bit(mask, u32::from(d.first).wrapping_sub(u32::from(d.division))))
    {
        return mask;
    }
    let window = (1_u64 << d.records) - 1;
    // GB returns uint (32 bits) even when its input is uint64; retain truncation.
    let extracted = (mask >> d.first) & window;
    let low = extracted.to_le_bytes();
    let extracted = u32::from_le_bytes([low[0], low[1], low[2], low[3]]);
    let shifted = (u64::from(extracted) << 1) | 1;
    // SB clears the window, then ORs the supplied bits without a second mask.
    (mask & !(window << d.first)) | (shifted << d.first)
}

/// Rust-owned scalar state; C++ owns every history/scratch element and operation.
pub struct Engine {
    mode: u8,
    mask: u64,
    month: u32,
    frames: Vec<Frame>,
    final_valid: bool,
}

impl Engine {
    pub fn new(mode: u8, token: usize, mask: u64, age: u32, month: u32) -> Self {
        assert!(mode <= 3);
        Self {
            mode,
            mask,
            month,
            frames: vec![Frame::new(token, age)],
            final_valid: false,
        }
    }

    pub fn describe(&mut self, descriptor: Descriptor) {
        let frame = self.frames.last_mut().unwrap();
        assert!(frame.descriptor.is_none());
        frame.descriptor = Some(descriptor);
    }

    pub fn phase(&mut self, month: u32) {
        assert_eq!(self.mode, 3);
        let frame = self.frames.last_mut().unwrap();
        assert!(matches!(frame.stage, Stage::AwaitPhase));
        frame.start = start(frame.descriptor.unwrap(), frame.age, month);
        frame.stage = Stage::Children;
    }

    pub fn complete_query(&mut self) -> bool {
        assert_eq!(self.mode, 3);
        let frame = self.frames.pop().unwrap();
        assert!(matches!(frame.stage, Stage::Complete));
        if let Some(parent) = self.frames.last_mut() {
            parent.valid |= frame.valid;
        } else {
            self.final_valid = frame.valid;
        }
        frame.valid
    }

    fn rotation_step(&mut self, index: usize, d: Descriptor) -> Option<Step> {
        match self.frames[index].stage {
            Stage::Enter => {
                self.frames[index].stage = Stage::AfterChild;
                if d.child != 0 {
                    self.frames.push(Frame::new(d.child, 0));
                }
            }
            Stage::AfterChild if self.mode == 0 => {
                self.mask = update(self.mask, d, self.month);
                self.frames.pop();
            }
            Stage::AfterChild => {
                if !self.month.is_multiple_of(u32::from(d.total_division)) {
                    self.frames.pop();
                    return None;
                }
                self.frames[index].stage = Stage::Leading;
                return Some(Step {
                    kind: 2,
                    first: u32::from(d.first),
                    last: u32::from(d.last),
                    ..Step::default()
                });
            }
            Stage::Leading => {
                if d.total_division == 1 {
                    self.frames[index].stage = Stage::Reset;
                    return Some(Step {
                        kind: 3,
                        first: u32::from(d.first).wrapping_sub(1),
                        target: u32::from(d.first),
                        ..Step::default()
                    });
                }
                self.frames[index].stage = Stage::Pop;
                let source = u32::from(d.first).wrapping_sub(u32::from(d.division));
                if has_bit(self.mask, source) {
                    return Some(Step {
                        kind: 5,
                        count: d.division,
                        first: source,
                        target: u32::from(d.first),
                        ..Step::default()
                    });
                }
            }
            Stage::Reset => {
                self.frames[index].stage = Stage::Pop;
                return Some(Step {
                    kind: 4,
                    ..Step::default()
                });
            }
            Stage::Pop => {
                self.frames.pop();
            }
            _ => unreachable!("invalid rotation/update stage"),
        }
        None
    }

    fn query_step(&mut self, index: usize, d: Descriptor, age: u32) -> Step {
        match self.frames[index].stage {
            Stage::Enter => {
                if aggregate(d, age) {
                    self.frames[index].stage = Stage::AwaitPhase;
                    return Step {
                        kind: 6,
                        count: d.division,
                        ..Step::default()
                    };
                }
                if age < u32::from(d.periods) {
                    let source = slot(d, age);
                    self.frames[index].valid = has_bit(self.mask, source);
                    self.frames[index].stage = Stage::Complete;
                    return Step {
                        kind: 9,
                        first: source,
                        ..Step::default()
                    };
                }
                Step {
                    kind: 10,
                    ..Step::default()
                }
            }
            Stage::Children => {
                let child_index = self.frames[index].child_index;
                if child_index < u32::from(d.division) {
                    self.frames[index].child_index += 1;
                    let age = self.frames[index].start.wrapping_add(child_index);
                    self.frames.push(Frame::new(d.child, age));
                    return Step {
                        kind: 7,
                        target: child_index,
                        ..Step::default()
                    };
                }
                self.frames[index].stage = Stage::Complete;
                Step {
                    kind: 8,
                    count: d.division,
                    ..Step::default()
                }
            }
            _ => unreachable!("query operation was not completed"),
        }
    }

    pub fn next(&mut self) -> Step {
        loop {
            let Some(index) = self.frames.len().checked_sub(1) else {
                return Step {
                    value: if self.mode == 0 {
                        self.mask
                    } else {
                        u64::from(self.final_valid)
                    },
                    ..Step::default()
                };
            };
            let Some(d) = self.frames[index].descriptor else {
                return Step {
                    kind: 1,
                    token: self.frames[index].token,
                    ..Step::default()
                };
            };
            if matches!(self.mode, 0 | 2) {
                if let Some(step) = self.rotation_step(index, d) {
                    return step;
                }
                continue;
            }
            let age = self.frames[index].age;
            if self.mode == 1 {
                if aggregate(d, age) {
                    self.frames[index] = Frame::new(d.child, start(d, age, self.month));
                    continue;
                }
                self.final_valid = age < u32::from(d.periods) && has_bit(self.mask, slot(d, age));
                self.frames.clear();
                continue;
            }
            return self.query_step(index, d, age);
        }
    }
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDRustHistoryDescriptor" ["probe"] => Descriptor {
        child,
        periods,
        records,
        first,
        last,
        division,
        total_division,
        child_periods,
        child_division,
    };
    "OpenTTDRustHistoryStep" ["probe"] => Step {
        kind,
        count,
        first,
        last,
        target,
        token,
        value,
    };
}
