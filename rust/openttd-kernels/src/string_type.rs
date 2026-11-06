/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Shared string-validation flags from `string_type.h`.
/// `StringValidationSettings{StringValidationSetting::ReplaceWithQuestionMark}`.
pub(crate) const REPLACE_WITH_QUESTION_MARK: u8 = 1;
/// `StringValidationSetting::AllowNewline`.
pub(crate) const ALLOW_NEWLINE: u8 = 2;
