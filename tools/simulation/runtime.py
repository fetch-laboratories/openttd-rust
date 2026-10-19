"""Export frozen game runtimes and once-built scenario adapters for CI workers."""

import hashlib
import json
import shutil
import struct
import subprocess
from functools import cache
from pathlib import Path

from build_identity import IDENTITY_NAME, read_identity, source_identity

from . import core

MANIFEST = "runtime-manifest.json"


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def elf_content(path):
    """Hash loadable bytes and allocated object sections, including layout.

    Only ELF header section-table location/count/name-index bytes are normalized:
    stripping debug sections changes these non-loader metadata fields. Program
    headers, all other mapped bytes and every allocated section remain exact.
    """
    data = Path(path).read_bytes()
    if data[:4] != b"\x7fELF" or data[4] not in (1, 2) or data[5] not in (1, 2):
        raise RuntimeError("Native export is not a supported ELF file")
    endian = "<" if data[5] == 1 else ">"
    wide = data[4] == 2
    header = struct.unpack_from(
        endian + ("HHIQQQIHHHHHH" if wide else "HHIIIIIHHHHHH"), data, 16
    )
    phoff, shoff, phsize, phcount, shsize, shcount, names_index = (
        header[4],
        header[5],
        header[8],
        header[9],
        header[10],
        header[11],
        header[12],
    )
    load = []
    normalized = bytearray(data)
    for start, end in ((40, 48), (60, 64)) if wide else ((32, 36), (48, 52)):
        normalized[start:end] = bytes(end - start)
    for index in range(phcount):
        values = struct.unpack_from(
            endian + ("IIQQQQQQ" if wide else "IIIIIIII"), data, phoff + index * phsize
        )
        if values[0] != 1:  # PT_LOAD
            continue
        offset, size = (values[2], values[5]) if wide else (values[1], values[4])
        load.append(
            {
                "header": list(values),
                "sha256": hashlib.sha256(
                    normalized[offset : offset + size]
                ).hexdigest(),
            }
        )
    sections = [
        struct.unpack_from(
            endian + ("IIQQQQIIQQ" if wide else "IIIIIIIIII"),
            data,
            shoff + index * shsize,
        )
        for index in range(shcount)
    ]
    names = sections[names_index]
    strings = data[names[4] : names[4] + names[5]]
    section_names = [
        strings[s[0] : strings.index(b"\0", s[0])].decode() for s in sections
    ]
    symbol_strings = {}

    def debug_section(index):
        return section_names[index].startswith((".debug", ".zdebug"))

    @cache
    def symbol(table, index):
        entry = sections[table]
        values = struct.unpack_from(
            endian + ("IBBHQQ" if wide else "IIIBBH"),
            data,
            entry[4] + index * entry[9],
        )
        name, info, other, target, value, size = (
            values
            if wide
            else (values[0], values[3], values[4], values[5], values[1], values[2])
        )
        if table not in symbol_strings:
            names = sections[entry[6]]
            symbol_strings[table] = data[names[4] : names[4] + names[5]]
        strings = symbol_strings[table]
        return {
            "name": strings[name : strings.index(b"\0", name)].decode(),
            "info": info,
            "other": other,
            "value": value,
            "size": size,
            "section": section_names[target] if 0 < target < len(sections) else target,
        }, target

    link_inputs = None
    if header[0] == 1:  # ET_REL: section bytes alone do not prove link semantics.
        symbols, relocations, groups, other_sections = [], [], [], []
        for index, section in enumerate(sections):
            name = section_names[index]
            if debug_section(index):
                continue
            if section[1] == 2:  # SHT_SYMTAB; strip-debug renumbers retained symbols.
                for entry in range(section[5] // section[9]):
                    record, target = symbol(index, entry)
                    if record["info"] >> 4 != 0 and not (
                        0 < target < len(sections) and debug_section(target)
                    ):  # Local symbols are resolved in relocation/group records;
                        # unused section symbols may have served debug references only.
                        symbols.append(record)
            elif section[1] in (4, 9):  # RELA/REL, resolved through their symbol table.
                if debug_section(section[7]):
                    continue
                entries = []
                for entry in range(section[5] // section[9]):
                    fmt = (
                        ("QQq" if wide else "IIi")
                        if section[1] == 4
                        else ("QQ" if wide else "II")
                    )
                    values = struct.unpack_from(
                        endian + fmt, data, section[4] + entry * section[9]
                    )
                    offset, info = values[:2]
                    entries.append(
                        {
                            "offset": offset,
                            "type": info & (0xFFFFFFFF if wide else 0xFF),
                            "symbol": symbol(section[6], info >> (32 if wide else 8))[
                                0
                            ],
                            "addend": values[2] if len(values) == 3 else None,
                        }
                    )
                relocations.append(
                    {
                        "name": name,
                        "target": section_names[section[7]],
                        "entries": entries,
                    }
                )
            elif section[1] == 17:  # SHT_GROUP/COMDAT indices also change on stripping.
                members = struct.unpack_from(
                    endian + "I" * (section[5] // 4), data, section[4]
                )
                groups.append(
                    {
                        "name": name,
                        "flags": members[0],
                        "signature": symbol(section[6], section[7])[0],
                        "members": [section_names[i] for i in members[1:]],
                    }
                )
            elif section[1] not in (
                0,
                3,
            ):  # Nondebug notes/flags can affect linking too.
                other_sections.append(
                    {
                        "name": name,
                        "type": section[1],
                        "flags": section[2],
                        "alignment": section[8],
                        "size": section[5],
                        "sha256": hashlib.sha256(
                            data[section[4] : section[4] + section[5]]
                            if section[1] != 8
                            else b""
                        ).hexdigest(),
                    }
                )
        semantics = {
            "symbols": symbols,
            "relocations": relocations,
            "groups": groups,
            "sections": other_sections,
        }
        link_inputs = {
            "sha256": hashlib.sha256(
                json.dumps(semantics, sort_keys=True).encode()
            ).hexdigest(),
            "symbols": len(symbols),
            "relocations": sum(len(r["entries"]) for r in relocations),
            "groups": len(groups),
            "sections": len(other_sections),
        }
    allocated = []
    for index, section in enumerate(sections):
        if not section[2] & 2:  # SHF_ALLOC, also checks relocatable widget objects.
            continue
        name = section_names[index]
        body = b"" if section[1] == 8 else data[section[4] : section[4] + section[5]]
        allocated.append(
            {
                "name": name,
                "type": section[1],
                "flags": section[2],
                "address": section[3],
                "size": section[5],
                "alignment": section[8],
                "sha256": hashlib.sha256(body).hexdigest(),
            }
        )
    return {
        "PT_LOAD_without_section_table_header_metadata": load,
        "SHF_ALLOC": allocated,
        "relocatable_link_inputs": link_inputs,
    }


def strip_debug(source, destination=None):
    """Strip debug data from an exported copy, proving loaded code/data is exact."""
    before = digest(source)
    proof = elf_content(source)
    command = ["objcopy", "--strip-debug", str(source)]
    if destination is not None:
        command.append(str(destination))
    subprocess.run(command, check=True)
    target = destination or source
    if elf_content(target) != proof:
        raise RuntimeError("Debug stripping changed loadable code/data or layout")
    return {
        "source_sha256": before,
        "sha256": digest(target),
        "command": command,
        "content_proof": proof,
    }


def candidate_identity(build, binary, checkout):
    """Bind the export to the canonical binary/configuration and current source."""
    identity = read_identity(build, binary)
    if (
        identity is None
        or not identity["source_stable"]
        or checkout["source_status"]
        or any(identity[key] != value for key, value in checkout.items())
    ):
        raise RuntimeError("Candidate binary/configuration/source identity is stale")
    return identity


def export(destination, migration):
    """Capture under build locks; adapters must come from these exact game objects."""
    from . import engines, fleet, orders, signals, stations, towns

    checkout = source_identity(core.ROOT)
    if checkout["source_status"]:
        raise RuntimeError("Frozen export requires a clean candidate checkout")
    destination.mkdir(parents=True)
    records = {}
    builds = {
        "reference": migration.REFERENCE_BUILD.resolve(),
        "candidate": core.ROOT / "build-rust",
    }
    for role, build in builds.items():
        lock = (
            migration.reference_lock(shared=True)
            if role == "reference"
            else migration.candidate_lock(build, shared=True)
        )
        with lock:
            source = build / ("openttd" if role == "reference" else "openttd-rust")
            if role == "candidate":
                identity = candidate_identity(build, source, checkout)
            runtime = destination / f"{role}-runtime"
            binary = core._copy_runtime(build, source, runtime)
            if role == "candidate":
                shutil.copy2(build / IDENTITY_NAME, runtime / IDENTITY_NAME)
            ordinary_source_sha256 = digest(binary)
            adapters = {}
            derivations = {}
            for module in (fleet, orders, engines, signals, stations, towns):
                folder = runtime / module.__name__.rsplit(".", 1)[1]
                executable, receipt = module.adapter(
                    build, folder, migration.environment()
                )
                if receipt["ordinary_binary_sha256"] != digest(binary):
                    raise RuntimeError("Exported adapter does not match frozen game")
                for object_file in folder.glob("*.o"):
                    object_file.unlink()
                derivation = strip_debug(executable)
                receipt["source_binary_sha256"] = receipt["binary_sha256"]
                receipt["binary_sha256"] = derivation["sha256"]
                derivations[folder.name] = derivation
                adapters[folder.name] = receipt
            ordinary = strip_debug(binary)
            for receipt in adapters.values():
                receipt["source_ordinary_binary_sha256"] = ordinary_source_sha256
                receipt["ordinary_binary_sha256"] = ordinary["sha256"]
            (runtime / "adapters.json").write_text(
                json.dumps(adapters, indent=2) + "\n"
            )
        records[role] = {
            "binary": str(binary.relative_to(destination)),
            "ordinary_derivation": ordinary,
            "adapter_derivations": derivations,
        }
    files = {
        str(path.relative_to(destination)): digest(path)
        for path in sorted(destination.rglob("*"))
        if path.is_file()
    }
    if source_identity(core.ROOT) != checkout:
        raise RuntimeError("Candidate source changed during frozen export")
    manifest = {
        "schema_version": 1,
        "candidate_commit": migration.git("rev-parse", "HEAD"),
        "candidate_build_identity": identity,
        "baseline": migration.BASELINE,
        "roles": records,
        "files": files,
    }
    (destination / MANIFEST).write_text(json.dumps(manifest, indent=2) + "\n")
    return manifest


def verify(destination, migration):
    manifest = json.loads((destination / MANIFEST).read_text())
    if manifest["schema_version"] != 1 or manifest["baseline"] != migration.BASELINE:
        raise RuntimeError("Frozen runtime baseline/schema differs")
    if manifest["candidate_commit"] != migration.git("rev-parse", "HEAD"):
        raise RuntimeError("Frozen runtime belongs to another candidate commit")
    identity = manifest["candidate_build_identity"]
    checkout = source_identity(core.ROOT)
    if (
        checkout["source_status"]
        or any(identity.get(key) != value for key, value in checkout.items())
        or identity["source_commit"] != manifest["candidate_commit"]
        or not identity["source_stable"]
    ):
        raise RuntimeError(
            "Frozen candidate source was not stable at the captured commit"
        )
    expected = manifest["files"]
    actual = {
        str(path.relative_to(destination)): digest(path)
        for path in sorted(destination.rglob("*"))
        if path.is_file()
        and path not in (destination / MANIFEST, destination / "shard-plan.json")
    }
    if actual != expected:
        raise RuntimeError("Frozen runtime files differ from the build manifest")
    if (
        manifest["roles"]["candidate"]["ordinary_derivation"]["source_sha256"]
        != identity["binary_sha256"]
        or json.loads((destination / "candidate-runtime" / IDENTITY_NAME).read_text())
        != identity
    ):
        raise RuntimeError(
            "Derived candidate differs from its canonical build identity"
        )
    for record in manifest["roles"].values():
        binary = destination / record["binary"]
        derivation = record["ordinary_derivation"]
        if (
            derivation["sha256"] != expected[record["binary"]]
            or elf_content(binary) != derivation["content_proof"]
        ):
            raise RuntimeError("Derived runtime differs from its code/data proof")
        adapters = json.loads((binary.parent / "adapters.json").read_text())
        for kind, derived in record["adapter_derivations"].items():
            executable = binary.parent / kind / f"openttd-{kind}"
            receipt = adapters[kind]
            if (
                derived["sha256"] != digest(executable)
                or derived["source_sha256"] != receipt["source_binary_sha256"]
                or receipt["binary_sha256"] != derived["sha256"]
                or receipt["ordinary_binary_sha256"] != derivation["sha256"]
                or receipt["source_ordinary_binary_sha256"]
                != derivation["source_sha256"]
                or elf_content(executable) != derived["content_proof"]
            ):
                raise RuntimeError(
                    "Derived adapter differs from its source/runtime/proof"
                )
    return manifest


def prepared_adapter(kind, role, binaries, builds, folder):
    """Use a bundle adapter, preserving the original compile/link provenance."""
    runtime = builds[role]
    receipt_file = runtime / "adapters.json"
    if not receipt_file.is_file():
        return None
    receipt = json.loads(receipt_file.read_text())[kind]
    source = runtime / kind / f"openttd-{kind}"
    if receipt["binary_sha256"] != digest(source) or receipt[
        "ordinary_binary_sha256"
    ] != digest(binaries[role]):
        raise RuntimeError(
            "Prepared adapter differs from frozen game or adapter manifest"
        )
    folder.mkdir(parents=True, exist_ok=True)
    binary = folder / source.name
    shutil.copy2(source, binary)
    return binary, receipt
