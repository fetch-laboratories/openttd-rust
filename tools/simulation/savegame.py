"""Save codec, input patches and semantic comparison."""

import fnmatch
import hashlib
import lzma
import re
import struct
from collections.abc import Iterable, Iterator, Mapping, Sequence
from pathlib import Path
from typing import Any, NotRequired, TypedDict, cast

from .models import Difference, Stats

MASKS = {
    ("DATE", "id"): "random savegame id generated for every new game",
    ("GLOG", "action/revision/revision.text"): "build revision string of each binary",
    (
        "GLOG",
        "action/revision/revision.modified",
    ): "whether each build tree was modified",
    (
        "GLOG",
        "action/revision/revision.newgrf",
    ): "OpenTTD version each build reports to NewGRFs (from its git revision)",
    # BaseConsist::round_trip_time has no initializer (base_consist.h:43), so a
    # vehicle built in a reused pool slot saves stale heap bytes until its first
    # measured round trip; the reference differs from itself here (#83).
    ("VEHS", "*/common/round_trip_time"): "uninitialized in the original",
}

# Known reference-vs-candidate divergences caused by existing ports, recorded by
# their exact first divergence. Each names its bug issue; fix the port to remove
# an entry, never widen it. Format: (scenario, chunk, field path) -> "#issue".
KNOWN_FAILURES = {}


class Corrupt(Exception):
    pass


class Field(TypedDict):
    """One table header entry; struct fields carry their own sub-header."""

    type: int
    key: str
    fields: NotRequired[list["Field"]]


Buffer = bytes | bytearray


class Chunk(TypedDict):
    kind: int
    elements: list[tuple[int, Buffer]]
    header: list[Field] | None
    raw: NotRequired[Buffer]
    span: tuple[int, int]
    noncanonical: int


Chunks = dict[str, Chunk]
# Decoded field values by path; each value's type follows the save's header.
Row = dict[str, Any]


class Reader:
    def __init__(self, data: Buffer, pos: int = 0, end: int | None = None) -> None:
        self.data, self.pos, self.end = data, pos, len(data) if end is None else end
        # Gammas whose bytes are not SlWriteSimpleGamma's minimal form for their value.
        self.noncanonical = 0

    def take(self, count: int) -> Buffer:
        if self.pos + count > self.end:
            raise Corrupt("read past end")
        value = self.data[self.pos : self.pos + count]
        self.pos += count
        return value

    def byte(self) -> int:
        return self.take(1)[0]

    def gamma(self) -> int:
        """Saveload SlReadSimpleGamma: 1 to 5 bytes, big endian, prefix coded."""
        start = self.pos
        first = self.byte()
        if first < 0x80:
            value = first
        elif first < 0xC0:
            value = ((first & 0x3F) << 8) | self.byte()
        elif first < 0xE0:
            value = ((first & 0x1F) << 16) | int.from_bytes(self.take(2), "big")
        elif first < 0xF0:
            value = ((first & 0x0F) << 24) | int.from_bytes(self.take(3), "big")
        elif first < 0xF8:
            value = int.from_bytes(self.take(4), "big")
        else:
            raise Corrupt("invalid gamma")
        self.noncanonical += self.data[start : self.pos] != gamma(value)
        return value


# SLE_FILE_* types: (struct format, size); 10 is STRING, 11 is STRUCT.
FILE_TYPES: dict[int, tuple[str, int]] = {
    1: (">b", 1),
    2: (">B", 1),
    3: (">h", 2),
    4: (">H", 2),
    5: (">i", 4),
    6: (">I", 4),
    7: (">q", 8),
    8: (">Q", 8),
    9: (">H", 2),
}
FILE_STRING, FILE_STRUCT, HAS_LENGTH = 10, 11, 0x10


def read_header(reader: Reader) -> list[Field]:
    """Table header: (type, key) pairs ending in type 0, then sub-headers of struct fields."""
    fields: list[Field] = []
    while True:
        kind = reader.byte()
        if kind == 0:
            break
        fields.append(
            {
                "type": kind,
                "key": reader.take(reader.gamma()).decode("utf-8", "surrogateescape"),
            }
        )
    for field in fields:
        if field["type"] & 0x0F == FILE_STRUCT:
            field["fields"] = read_header(reader)
    return fields


def read_value(reader: Reader, kind: int) -> int | str:
    base = kind & 0x0F
    if base == FILE_STRING:
        # surrogateescape keeps distinct invalid byte sequences distinct.
        return reader.take(reader.gamma()).decode("utf-8", "surrogateescape")
    fmt, size = FILE_TYPES[base]
    return struct.unpack(fmt, reader.take(size))[0]


def read_object(reader: Reader, fields: list[Field], prefix: str, out: Row) -> None:
    """Flatten one object into out[path] = value, following SlObject's table encoding."""
    for field in fields:
        path = prefix + field["key"]
        kind, base = field["type"], field["type"] & 0x0F
        if base == FILE_STRUCT:
            for index in range(reader.gamma()):
                read_object(reader, field["fields"], f"{path}[{index}]/", out)
        elif base == FILE_STRING or not kind & HAS_LENGTH:
            out[path] = read_value(reader, kind)
        else:
            out[path] = tuple(read_value(reader, kind) for _ in range(reader.gamma()))


def read_save(path: Path) -> Chunks:
    """Return {chunk id: chunk} for an uncompressed OTTN save."""
    return parse_save(path.read_bytes(), path.name)


def parse_save(data: Buffer, name: str = "save") -> Chunks:
    if data[:4] != b"OTTN":
        raise Corrupt(f"{name} is not an uncompressed save")
    reader, chunks = Reader(data, 8), Chunks()
    while True:
        chunk_start = reader.pos
        cid = reader.take(4)
        if cid == b"\0\0\0\0":
            return chunks
        mode = reader.byte()
        kind = mode & 0x0F
        chunk: Chunk = {
            "kind": kind,
            "elements": [],
            "header": None,
            "span": (0, 0),
            "noncanonical": 0,
        }
        before = reader.noncanonical
        if kind == 0:  # CH_RIFF
            length = int.from_bytes(reader.take(3), "big") + ((mode >> 4) << 24)
            chunk["raw"] = reader.take(length)
        else:
            if kind in (3, 4):  # CH_TABLE, CH_SPARSE_TABLE
                length = reader.gamma() - 1
                header = Reader(reader.take(length))
                chunk["header"] = read_header(header)
                reader.noncanonical += header.noncanonical
            index = 0
            while True:
                length = reader.gamma()
                if length == 0:
                    break
                element = Reader(reader.take(length - 1))
                if kind in (2, 4):  # sparse: index precedes the element body
                    index = element.gamma()
                    reader.noncanonical += element.noncanonical
                # SlIterateArray skips empty non-sparse slots; a sparse element
                # always counts because its length includes the index.
                if kind in (2, 4) or element.pos < len(element.data):
                    chunk["elements"].append((index, element.data[element.pos :]))
                index += 1
        chunk["span"] = (chunk_start, reader.pos)
        # Chunk, header, element and sparse-index lengths decode to integers, so a
        # non-minimal gamma would otherwise compare equal.
        chunk["noncanonical"] = reader.noncanonical - before
        chunks[cid.decode("latin-1")] = chunk


def decode_element(chunk: Chunk, body: Buffer) -> Row | None:
    if chunk["header"] is None:
        return None
    reader, out = Reader(body), Row()
    try:
        read_object(reader, chunk["header"], "", out)
    except (Corrupt, struct.error, KeyError):
        return None
    # Chunks whose handlers append data beyond the described fields fall back to bytes.
    return out if reader.pos == len(body) else None


def mask_for(cid: str, path: str) -> str | None:
    """Return the MASKS pattern covering this field, if any."""
    generic = re.sub(r"\[\d+\]", "", path)
    return next(
        (
            pattern
            for chunk, pattern in MASKS
            if cid == chunk and fnmatch.fnmatchcase(generic, pattern)
        ),
        None,
    )


def compare_chunk(
    cid: str, ref: Chunk, cand: Chunk, stats: Stats
) -> Iterator[tuple[str, str, object, object]]:
    """Yield (element, field, reference, candidate) differences in one chunk."""
    if ref["kind"] != cand["kind"]:
        yield ("", "chunk type", ref["kind"], cand["kind"])
        return
    if ref["noncanonical"] != cand["noncanonical"]:
        yield ("", "gamma encoding", ref["noncanonical"], cand["noncanonical"])
    if ref["kind"] == 0:
        a, b = ref["raw"], cand["raw"]
        stats["elements"] += 1
        if a != b:
            if len(a) != len(b):
                yield ("", "length", len(a), len(b))
            for offset in range(min(len(a), len(b))):
                if a[offset] != b[offset]:
                    yield (f"byte {offset}", "", a[offset], b[offset])
        return
    if ref["header"] != cand["header"]:
        yield ("", "table header", "differs", "differs")
        return
    left, right = dict(ref["elements"]), dict(cand["elements"])
    for index in sorted(set(left) | set(right)):
        stats["elements"] += 1
        a, b = left.get(index), right.get(index)
        if a == b:
            continue
        if a is None or b is None:
            yield (
                str(index),
                "element",
                "present" if a is not None else "absent",
                "present" if b is not None else "absent",
            )
            continue
        fields_a, fields_b = decode_element(ref, a), decode_element(cand, b)
        if fields_a is None or fields_b is None:
            # Undecodable element: masks cannot apply, so any difference counts.
            yield (str(index), "bytes", a.hex()[:64], b.hex()[:64])
            continue
        if fields_a == fields_b:
            # Equal fields from different bytes differ only in gamma encoding.
            yield (str(index), "encoding", a.hex()[:64], b.hex()[:64])
            continue
        for path in sorted(set(fields_a) | set(fields_b)):
            va, vb = fields_a.get(path), fields_b.get(path)
            if va == vb:
                continue
            pattern = mask_for(cid, path)
            if pattern is None:
                yield (str(index), path, va, vb)
            else:
                stats["masked"][f"{cid}:{pattern}"] = (
                    stats["masked"].get(f"{cid}:{pattern}", 0) + 1
                )


def failure_key(
    scenario: str, cid: str, element: str, field: str
) -> tuple[str, str, str]:
    """KNOWN_FAILURES key: the field path without indices, or the element for byte-level differences."""
    return (
        scenario,
        cid,
        re.sub(r"\[\d+\]", "", field) if field not in ("", "bytes") else element,
    )


def compare_saves(
    ref_path: Path, cand_path: Path, scenario: str, limit: int, stats: Stats
) -> list[Difference]:
    """Return difference records; stops after `limit` differences not in KNOWN_FAILURES."""
    ref, cand = read_save(ref_path), read_save(cand_path)
    records: list[Difference] = []
    unknown = 0
    for cid in sorted(set(ref) | set(cand)):
        stats["chunks"] += 1
        diffs: Iterable[tuple[str, str, object, object]]
        if cid not in ref or cid not in cand:
            diffs = [("", "chunk", cid in ref, cid in cand)]
        else:
            diffs = compare_chunk(cid, ref[cid], cand[cid], stats)
        for element, field, ref_value, cand_value in diffs:
            record: Difference = {
                "snapshot": ref_path.name,
                "chunk": cid,
                "element": element,
                "field": field,
                "reference": repr(ref_value),
                "candidate": repr(cand_value),
            }
            issue = KNOWN_FAILURES.get(failure_key(scenario, cid, element, field))
            if issue:
                record["issue"] = issue
            else:
                unknown += 1
            records.append(record)
            if unknown >= limit:
                return records
    return records


def save_moment(path: Path) -> tuple[int, int, int]:
    """(date, date_fract, tick_counter) at which a save was written."""
    chunk = read_save(path)["DATE"]
    fields = cast(Row, decode_element(chunk, chunk["elements"][0][1]))
    return fields["date"], fields["date_fract"], fields["tick_counter"]


def first_difference(
    a: Sequence[object], b: Sequence[object]
) -> tuple[int, object, object]:
    index = next(
        i
        for i, (x, y) in enumerate(zip([*a, None], [*b, None], strict=False))
        if x != y
    )
    return (
        index,
        (a[index] if index < len(a) else None),
        (b[index] if index < len(b) else None),
    )


def field_spans(body: Buffer, header: list[Field]) -> dict[str, tuple[int, int, int]]:
    """Locate table values for input patches without changing semantic decoding."""
    reader, spans = Reader(body), dict[str, tuple[int, int, int]]()

    def visit(fields: list[Field], prefix: str) -> None:
        for field in fields:
            kind, path = field["type"], prefix + field["key"]
            if kind & 15 == FILE_STRUCT:
                for index in range(reader.gamma()):
                    visit(field["fields"], f"{path}[{index}]/")
            else:
                begin = reader.pos
                if kind & 15 == FILE_STRING:
                    reader.take(reader.gamma())
                else:
                    count = reader.gamma() if kind & HAS_LENGTH else 1
                    reader.take(FILE_TYPES[kind & 15][1] * count)
                spans[path] = (begin, reader.pos, kind)

    visit(header, "")
    if reader.pos != len(body):
        raise RuntimeError("prepared VEHS element has undescribed bytes")
    return spans


def gamma(value: int) -> bytes:
    """Saveload SlWriteSimpleGamma, the inverse of Reader.gamma."""
    for size, limit, prefix in (
        (1, 128, 0),
        (2, 16384, 0x8000),
        (3, 2097152, 0xC00000),
        (4, 268435456, 0xE0000000),
    ):
        if value < limit:
            return (value | prefix).to_bytes(size, "big")
    return b"\xf0" + value.to_bytes(4, "big")


def rows(source: Path | Chunks, cid: str = "VEHS") -> dict[int, Row]:
    """Decoded table elements by index from a save path or its read_save chunks."""
    chunk = (source if isinstance(source, dict) else read_save(source))[cid]
    result = {index: decode_element(chunk, body) for index, body in chunk["elements"]}
    if any(row is None for row in result.values()):
        raise RuntimeError(f"undecodable {cid} element")
    return cast(dict[int, Row], result)


def strip_glog(data: Buffer, target: Path) -> Chunks:
    """Write save bytes without optional GLOG history; other chunks keep every byte.

    Input fixtures only: a reference-produced revision log would make only the
    candidate append a load-revision event. Output GLOG is compared normally."""
    chunks = parse_save(data)
    begin, end = chunks["GLOG"]["span"] if "GLOG" in chunks else (0, 0)
    normalized = bytes(data[:begin] + data[end:])
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(normalized)
    check = read_save(target)
    if set(check) != set(chunks) - {"GLOG"} or any(
        data[slice(*chunks[cid]["span"])] != normalized[slice(*check[cid]["span"])]
        for cid in check
    ):
        raise RuntimeError(f"{target.name} changed outside optional GLOG history")
    return check


def normalize(source: Path, directory: Path) -> tuple[Path, dict[str, Any]]:
    """Keep every original chunk byte except optional GLOG history."""
    packed = source.read_bytes()
    data = (
        b"OTTN" + packed[4:8] + lzma.decompress(packed[8:])
        if packed[:4] == b"OTTX"
        else packed
    )
    directory.mkdir(parents=True, exist_ok=True)
    (directory / "owner-uncompressed.sav").write_bytes(data)
    normalized = directory / "rail-input.sav"
    check = strip_glog(data, normalized)
    return normalized, {
        "source": str(source),
        "source_sha256": hashlib.sha256(packed).hexdigest(),
        "normalized_sha256": hashlib.sha256(normalized.read_bytes()).hexdigest(),
        "removed_chunk": "GLOG",
        "typed_edits": [],
        "unchanged_chunks": sorted(check),
    }


EMPTY_AI = b"\0\0\xff\xff\xff\xff"  # AIPL slot without a configured AI


def ai_config(name: bytes) -> bytes:
    """AIPL configuration prefix: script name, empty settings, version 1."""
    return gamma(len(name)) + name + b"\0" + struct.pack(">I", 1)


def set_ai_slots(data: Buffer, slots: dict[int, Buffer]) -> bytes:
    """Replace AIPL elements by company index; other chunks keep every byte."""
    chunks = parse_save(data)
    chunk = chunks["AIPL"]
    if chunk["kind"] != 3 or chunk["header"] != [
        {"type": 26, "key": "name"},
        {"type": 26, "key": "settings"},
        {"type": 6, "key": "version"},
    ]:
        raise RuntimeError("unexpected AIPL schema")
    elements = dict(chunk["elements"]) | slots
    reader = Reader(data, chunk["span"][0] + 5)
    reader.take(reader.gamma() - 1)
    body = b"".join(
        gamma(len(elements.get(index, b"")) + 1) + elements.get(index, b"")
        for index in range(max(elements) + 1)
    )
    prepared = bytes(data[: reader.pos] + body + b"\0" + data[chunk["span"][1] :])
    after = parse_save(prepared)
    if set(after) != set(chunks) or any(
        data[slice(*chunks[cid]["span"])] != prepared[slice(*after[cid]["span"])]
        for cid in set(chunks) - {"AIPL"}
    ):
        raise RuntimeError("AI configuration changed another chunk")
    return prepared


def patch(
    source: Path,
    target: Path,
    changes: Mapping[str, Mapping[int, Mapping[str, Any]]],
    ai: str | None = None,
) -> dict[str, Any]:
    """Only declared typed input fields, AIPL company 1 and optional GLOG change."""
    original, chunks = source.read_bytes(), read_save(source)
    work: Buffer = bytearray(original)
    allowed: set[int] = set()
    assignments: list[list[object]] = []
    for cid, byindex in changes.items():
        chunk = chunks[cid]
        if chunk["kind"] not in (3, 4):
            raise RuntimeError(f"unexpected input table {cid}")
        reader = Reader(original, chunk["span"][0] + 5)
        reader.take(reader.gamma() - 1)
        index, seen = 0, set()
        while length := reader.gamma():
            end = reader.pos + length - 1
            if chunk["kind"] == 4:
                index = reader.gamma()
            begin = reader.pos
            body = reader.take(end - begin)
            if index in byindex:
                fields, locations = (
                    cast(Row, decode_element(chunk, body)),
                    field_spans(body, cast(list[Field], chunk["header"])),
                )
                for key, value in byindex[index].items():
                    lo, hi, kind = locations[key]
                    fmt, width = FILE_TYPES[kind & 15]
                    vals = value if kind & HAS_LENGTH else (value,)
                    at = begin + lo
                    if kind & HAS_LENGTH:
                        array = Reader(body, lo)
                        if array.gamma() != len(vals):
                            raise RuntimeError("input array size differs")
                        at = begin + array.pos
                    if at + len(vals) * width != begin + hi:
                        raise RuntimeError("input field width differs")
                    for value_index, scalar in enumerate(vals):
                        struct.pack_into(fmt, work, at + value_index * width, scalar)
                    allowed.update(range(at, begin + hi))
                    assignments.append([cid, index, key, fields.get(key), value])
                seen.add(index)
            index += 1
        if seen != set(byindex):
            raise RuntimeError("input omitted edited element")
    if any(
        a != b and index not in allowed
        for index, (a, b) in enumerate(zip(original, work, strict=True))
    ):
        raise RuntimeError("input changed undeclared bytes")
    if ai is not None:
        if ai == "human" and dict(chunks["AIPL"]["elements"])[0] != EMPTY_AI:
            raise RuntimeError("company zero is not an empty human AI slot")
        config = ai_config(b"MigrationDisasters") * 2 + b"\0"
        work = set_ai_slots(work, {1: EMPTY_AI if ai == "human" else config})
        assignments.append(["AIPL", 1, "configuration", ai])
    after, patched = strip_glog(work, target), target.read_bytes()
    for cid in set(after) - set(changes) - ({"AIPL"} if ai is not None else set()):
        if original[slice(*chunks[cid]["span"])] != patched[slice(*after[cid]["span"])]:
            raise RuntimeError(f"input changed unrelated {cid}")
    return {
        "source": str(source),
        "source_sha256": hashlib.sha256(original).hexdigest(),
        "sha256": hashlib.sha256(patched).hexdigest(),
        "assignments": assignments,
        "removed_chunk": "GLOG",
    }
