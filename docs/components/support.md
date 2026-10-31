# Support

<!-- Add a new entry at the end of this file. -->

## StringConsumer integer parsing

- Rust owns: Integer base selection, digit scanning, conversion, clamping and lexical skipping.
- C++ keeps: Templates, cursor updates, adapters and logging.
- Checks: `python3 tools/run-comparisons.py`.
- PRs: #9.

## Alternating-iterator traversal

- Rust owns: Initial position, selectors, advancement, side selection, end transitions and position comparison. It needs no random access, and stable noncontiguous iterators stay supported after insertion.
- C++ keeps: Typed iterators, range assertions, dereferencing and container lifetimes.
- Checks: Public-interface tests.
- PRs: #13.

## StringConsumer byte algorithms

- Rust owns: Little-endian reads, bounded read/skip decisions, prefix matching, searches and separator decisions.
- C++ keeps: Conversions, views, cursor and diagnostics, the in-place pair, owning containers, allocators, escape parsing and encoded-string transformation.
- Checks: `python3 tools/run-comparisons.py`.
- Limits: Nonempty search/set/separator assertions are preconditions; release behavior outside them, including empty-separator loops, is not claimed.
- PRs: #18.

## StringBuilder numeric byte encoders

- Rust owns: `PutUint8`, `PutUint16LE`, `PutUint32LE`, `PutUint64LE` and integral `PutIntegerBase` encoding.
- C++ keeps: Virtual `PutBuffer` and sinks.
- Checks: `python3 tools/run-comparisons.py`.
- Limits: Invalid bases are outside the original 2..36 `std::to_chars` precondition.
- PRs: #17.

## Encoded-string compatibility and parameter rewriting

- Rust owns: `FixSCCEncoded`, `FixSCCEncodedNegative`, `EncodedString::ReplaceParam` and `GetEncodedStringWithArgs` serialization.
- C++ keeps: Save-version dispatch, general decoding, rendering, ScriptText encoding and sanitation.
- Checks: `python3 tools/encoded-comparison.py`.
- PRs: #24.

## Byte-string utilities

- Rust owns: Case-insensitive comparison and search, lowercase conversion, hexadecimal encoding/decoding and trim scanning.
- C++ keeps: String owners, views, erases, equal-prefix length policy, ICU and platform collation, validation, in-place replacement and `StringIterator` backends.
- Checks: `python3 tools/byte-strings-comparison.py`.
- Limits: Only the portable natural-contains fallback calls these helpers. Negative-char uppercase inputs other than EOF are outside portable C's domain; only observed native behavior is matched (#26).
- PRs: #27.

## UTF-8 codec and byte positions

- Rust owns: UTF-8 codec, `IsUtf8Part`, iterator stepping and `GetIterAtByte` normalization.
- C++ keeps: Leading ASCII fast paths, view and iterator facade.
- Checks: `python3 tools/utf8-comparison.py`.
- Limits: Game logging and Unicode rendering are outside the comparison.
- PRs: #10.

## Rounded square root and runtime integer saturation

- Rust owns: `IntSqrt` and runtime `SoftClamp`.
- C++ keeps: `ClampTo` (#276) and `DivideApprox`.
- Checks: `python3 tools/math-comparison.py` (macOS CI: `--build build`).
- Limits: `DivideApprox` has potentially overflowing signed intermediates requiring separate work.
- PRs: #36.

## Generic history structural engine

- Rust owns: History validity, rotation scheduling and query traversal.
- C++ keeps: `HistoryRange`, typed `HistoryData`, `SumHistory` specializations, graph fillers and `GetAndResetAccumulatedAverage`.
- Checks: `python3 tools/history-comparison.py`.
- PRs: #32.

## Nested widget descriptor parser

- Rust owns: `MakeNWidgets` and `MakeWindowNWidgetTree` descriptor parsing and tree composition.
- C++ keeps: Single-part factory, attribute operations, widget classes, owners, generators, constexpr builders and public signatures.
- Checks: `python3 tools/widget-parser-comparison.py`.
- Limits: Rendering, layout, events and arbitrary malformed descriptors are outside the comparison.
- PRs: #49.

## Coupled string validation and borrowed in-place replacement

- Rust owns: `StrMakeValid` policy, `StrValid` scanning and `InPlaceBuilder` write progression/copy.
- C++ keeps: String owners, cursor commit and fatal dispatch.
- Checks: `python3 tools/utf8-comparison.py`.
- Limits: Game logging and Unicode rendering are outside the comparison.
- PRs: #63.
