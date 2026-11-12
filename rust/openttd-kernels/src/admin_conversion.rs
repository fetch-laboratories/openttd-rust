/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Admin conversion traversal; typed VM/JSON effects execute between Rust calls.

use crate::json;
use crate::script_host::SQUIRREL_MAX_DEPTH;
use crate::squirrel::SQObjectType;

/// Scalar instruction whose C++ execution may invoke user code or throw.
#[repr(C)]
#[derive(Default)]
pub struct Action {
    /// Stable LIFO typed-frame slot, not a pointer.
    pub frame: u64,
    /// Signed VM index, or original saved stack top.
    pub index: i64,
    /// Operation pinned by `admin_conversion_ffi.h`.
    pub kind: u8,
    /// Scalar/container/cleanup selector.
    pub argument: u8,
}

const DONE: u8 = 0;
const INSPECT_OUT: u8 = 1;
const INSPECT_IN: u8 = 2;
const COPY_OUT_SCALAR: u8 = 3;
const PUSH_IN_SCALAR: u8 = 4;
const BEGIN_OUT: u8 = 5;
const BEGIN_IN: u8 = 6;
const NEXT_OUT: u8 = 7;
const CHECK_IN_ITERATOR: u8 = 8;
const COPY_OUT_KEY: u8 = 9;
const PUSH_IN_KEY: u8 = 10;
const CHILD_OUT: u8 = 11;
const CHILD_IN: u8 = 12;
const POP_PAIR: u8 = 13;
const POP_ITERATOR: u8 = 14;
const COMMIT_OUT_ARRAY: u8 = 15;
const COMMIT_OUT_TABLE: u8 = 16;
const COMMIT_IN_ARRAY: u8 = 17;
const COMMIT_IN_TABLE: u8 = 18;
const ADVANCE_IN_ITERATOR: u8 = 19;
const RELEASE_FRAME: u8 = 20;
const ERROR_DEPTH: u8 = 21;
const ERROR_OUT_TYPE: u8 = 22;
const ERROR_ROOT: u8 = 23;
const ERROR_IN_TYPE: u8 = 24;
const SAVE_TOP: u8 = 25;
const RESTORE_TOP: u8 = 26;
const PUSH_NULL: u8 = 27;

/// Scalar selector: null.
const SCALAR_NULL: u8 = 0;
/// Scalar selector: boolean.
const SCALAR_BOOL: u8 = 1;
/// Scalar selector: string.
const SCALAR_STRING: u8 = 2;
/// Scalar selector: integer.
const SCALAR_INTEGER: u8 = 3;
/// Release selector: destroy the child, then its parent's copied table key.
const RELEASE_KEY: u8 = 1;

#[derive(Clone, Copy)]
enum Stage {
    Enter,
    Inspected,
    Scalar,
    Begun,
    Iterated,
    /// The pair's key is copied; the parent keeps this stage while its child converts.
    Key,
    ChildFinished(bool),
    PairPopped(bool),
    Committed,
    Released,
    IteratorPopped(bool),
    Failed,
    InCommitted,
    InAdvanced,
}

struct Frame {
    index: i64,
    depth: i32,
    table: bool,
    stage: Stage,
}

#[derive(Clone, Copy)]
enum Control {
    Root,
    RootInspected,
    TopSaved,
    Walk,
    RootError,
    RootNull,
    Finished(bool),
    Restore,
    Invalid,
    InvalidNull,
    Done(bool),
}

/// One conversion: `ScriptAdminMakeJSON` when outgoing, `ScriptEventAdminPort::GetObject` with
/// `ScriptEventAdminPortReadValue` when incoming.
pub struct Engine {
    incoming: bool,
    frames: Vec<Frame>,
    control: Control,
    top: i64,
}

/// The frame slot as the native `size_t` index it stands for.
fn frame_slot(slot: usize) -> u64 {
    slot as u64
}

impl Engine {
    pub fn new(incoming: bool, index: i64, depth: i32) -> Self {
        Self {
            incoming,
            frames: vec![Frame {
                index,
                depth,
                table: false,
                stage: Stage::Enter,
            }],
            control: if incoming {
                Control::Root
            } else {
                Control::Walk
            },
            top: 0,
        }
    }

    fn action(&self, kind: u8, argument: u8) -> Action {
        let slot = self.frames.len().saturating_sub(1);
        Action {
            frame: frame_slot(slot),
            index: self.frames.last().map_or(0, |frame| frame.index),
            kind,
            argument,
        }
    }

    fn finish(&mut self, result: bool) -> Option<Action> {
        let slot = self.frames.len() - 1;
        self.frames.pop();
        if let Some(parent) = self.frames.last_mut() {
            parent.stage = Stage::ChildFinished(result);
            if !self.incoming {
                return None;
            }
        } else {
            self.control = Control::Finished(result);
        }
        Some(Action {
            frame: frame_slot(slot),
            kind: RELEASE_FRAME,
            ..Action::default()
        })
    }

    fn advance_control(&mut self, response: u64) -> Option<Action> {
        match self.control {
            Control::Root => {
                self.control = Control::RootInspected;
                return Some(self.action(INSPECT_IN, 0));
            }
            Control::RootInspected => {
                if response == u64::from(json::value_t::OBJECT) {
                    self.control = Control::TopSaved;
                    return Some(self.action(SAVE_TOP, 0));
                }
                self.control = Control::RootError;
                return Some(self.action(ERROR_ROOT, 0));
            }
            Control::TopSaved => {
                self.top = i64::from_ne_bytes(response.to_ne_bytes());
                self.control = Control::Walk;
            }
            Control::RootError => {
                self.control = Control::RootNull;
                return Some(self.action(PUSH_NULL, 0));
            }
            Control::RootNull => {
                self.frames.clear();
                self.control = Control::Done(false);
                return Some(Action {
                    kind: RELEASE_FRAME,
                    ..Action::default()
                });
            }
            Control::Finished(result) => {
                if self.incoming && !result {
                    /* Rewind the stack, removing anything that might be left on top. */
                    self.control = Control::Restore;
                    return Some(Action {
                        kind: RESTORE_TOP,
                        index: self.top,
                        ..Action::default()
                    });
                }
                self.control = Control::Done(result);
            }
            Control::Restore => {
                self.control = Control::Invalid;
                return Some(Action {
                    kind: ERROR_IN_TYPE,
                    ..Action::default()
                });
            }
            Control::Invalid => {
                self.control = Control::InvalidNull;
                return Some(Action {
                    kind: PUSH_NULL,
                    ..Action::default()
                });
            }
            Control::InvalidNull => self.control = Control::Done(false),
            Control::Done(result) => {
                return Some(Action {
                    kind: DONE,
                    argument: u8::from(result),
                    ..Action::default()
                });
            }
            Control::Walk => {}
        }
        None
    }

    fn enter(&mut self, slot: usize) -> Action {
        if !self.incoming && self.frames[slot].depth.cast_unsigned() == SQUIRREL_MAX_DEPTH {
            // SQUIRREL_MAX_DEPTH = 25
            self.frames[slot].stage = Stage::Failed;
            return self.action(ERROR_DEPTH, 0);
        }
        self.frames[slot].stage = Stage::Inspected;
        self.action(
            if self.incoming {
                INSPECT_IN
            } else {
                INSPECT_OUT
            },
            0,
        )
    }

    /// The type switch: the exact raw type values the original's cases name.
    fn inspect(&mut self, slot: usize, response: u64) -> Option<Action> {
        let scalar = if self.incoming {
            match u8::try_from(response) {
                Ok(json::value_t::NULL) => Some(SCALAR_NULL),
                Ok(json::value_t::BOOLEAN) => Some(SCALAR_BOOL),
                Ok(json::value_t::STRING) => Some(SCALAR_STRING),
                Ok(json::value_t::NUMBER_INTEGER | json::value_t::NUMBER_UNSIGNED) => {
                    Some(SCALAR_INTEGER)
                }
                _ => None,
            }
        } else {
            match u32::try_from(response).map(SQObjectType) {
                Ok(SQObjectType::OT_NULL) => Some(SCALAR_NULL),
                Ok(SQObjectType::OT_BOOL) => Some(SCALAR_BOOL),
                Ok(SQObjectType::OT_STRING) => Some(SCALAR_STRING),
                Ok(SQObjectType::OT_INTEGER) => Some(SCALAR_INTEGER),
                _ => None,
            }
        };
        if let Some(kind) = scalar {
            self.frames[slot].stage = Stage::Scalar;
            return Some(self.action(
                if self.incoming {
                    PUSH_IN_SCALAR
                } else {
                    COPY_OUT_SCALAR
                },
                kind,
            ));
        }
        let container = if self.incoming {
            match u8::try_from(response) {
                Ok(json::value_t::OBJECT) => Some(true),
                Ok(json::value_t::ARRAY) => Some(false),
                _ => None,
            }
        } else {
            match u32::try_from(response).map(SQObjectType) {
                Ok(SQObjectType::OT_TABLE) => Some(true),
                Ok(SQObjectType::OT_ARRAY) => Some(false),
                _ => None,
            }
        };
        if let Some(table) = container {
            self.frames[slot].table = table;
            self.frames[slot].stage = Stage::Begun;
            return Some(self.action(
                if self.incoming { BEGIN_IN } else { BEGIN_OUT },
                u8::from(table),
            ));
        }
        self.frames[slot].stage = Stage::Failed;
        if !self.incoming {
            return Some(self.action(ERROR_OUT_TYPE, 0));
        }
        /* These types are not supported by Squirrel. */
        None
    }

    /// The loop condition: `sq_next(vm, index - 1)` outgoing, the next item incoming.
    fn iterate(&mut self, slot: usize) -> Action {
        self.frames[slot].stage = Stage::Iterated;
        if self.incoming {
            return self.action(CHECK_IN_ITERATOR, 0);
        }
        let mut action = self.action(NEXT_OUT, 0);
        action.index = action.index.wrapping_sub(1);
        action
    }

    fn iterated(&mut self, slot: usize, response: u64) -> Option<Action> {
        if response == 0 {
            if self.incoming {
                return self.finish(true);
            }
            self.frames[slot].stage = Stage::IteratorPopped(true);
            return Some(self.action(POP_ITERATOR, 0));
        }
        self.frames[slot].stage = Stage::Key;
        if self.frames[slot].table {
            return Some(self.action(
                if self.incoming {
                    PUSH_IN_KEY
                } else {
                    COPY_OUT_KEY
                },
                0,
            ));
        }
        None
    }

    fn child(&mut self, slot: usize) -> Action {
        let depth = if self.incoming {
            0
        } else {
            self.frames[slot].depth.wrapping_add(1)
        };
        self.frames.push(Frame {
            index: -1,
            depth,
            table: false,
            stage: Stage::Enter,
        });
        self.action(if self.incoming { CHILD_IN } else { CHILD_OUT }, 0)
    }

    fn child_finished(&mut self, slot: usize, result: bool) -> Option<Action> {
        if self.incoming {
            if result {
                self.frames[slot].stage = Stage::InCommitted;
                return Some(self.action(
                    if self.frames[slot].table {
                        COMMIT_IN_TABLE
                    } else {
                        COMMIT_IN_ARRAY
                    },
                    0,
                ));
            }
            return self.finish(false);
        }
        self.frames[slot].stage = Stage::PairPopped(result);
        Some(self.action(POP_PAIR, 0))
    }

    fn pair_popped(&mut self, slot: usize, result: bool) -> Action {
        if result {
            self.frames[slot].stage = Stage::Committed;
            return self.action(
                if self.frames[slot].table {
                    COMMIT_OUT_TABLE
                } else {
                    COMMIT_OUT_ARRAY
                },
                0,
            );
        }
        self.frames[slot].stage = Stage::IteratorPopped(false);
        self.action(POP_ITERATOR, 0)
    }

    fn release_child(&mut self, slot: usize, stage: Stage) -> Action {
        self.frames[slot].stage = stage;
        Action {
            frame: frame_slot(slot + 1),
            kind: RELEASE_FRAME,
            argument: RELEASE_KEY,
            ..Action::default()
        }
    }

    pub fn advance(&mut self, response: u64) -> Action {
        loop {
            if let Some(action) = self.advance_control(response) {
                return action;
            }
            if !matches!(self.control, Control::Walk) {
                continue;
            }
            let slot = self.frames.len() - 1;
            let action = match self.frames[slot].stage {
                Stage::Enter => Some(self.enter(slot)),
                Stage::Inspected => self.inspect(slot, response),
                Stage::Scalar | Stage::IteratorPopped(true) => self.finish(true),
                Stage::Failed => self.finish(false),
                Stage::Begun | Stage::Released => Some(self.iterate(slot)),
                Stage::Iterated => self.iterated(slot, response),
                Stage::Key => Some(self.child(slot)),
                Stage::ChildFinished(result) => self.child_finished(slot, result),
                Stage::PairPopped(result) => Some(self.pair_popped(slot, result)),
                Stage::Committed => Some(self.release_child(slot, Stage::Released)),
                Stage::IteratorPopped(false) => Some(self.release_child(slot, Stage::Failed)),
                Stage::InCommitted => {
                    self.frames[slot].stage = Stage::InAdvanced;
                    Some(self.action(ADVANCE_IN_ITERATOR, 0))
                }
                Stage::InAdvanced => {
                    self.frames[slot].stage = Stage::Begun;
                    None
                }
            };
            if let Some(action) = action {
                return action;
            }
        }
    }
}
