#!/usr/bin/env python3
"""Compare only identified ScriptList owner, real-VM and persistence gaps.

Full unchanged game regressions are primary; this closes only listed gaps.
Limits: command disabling uses a host bool sentinel; save/clone TileList bodies
are extracted unchanged without world population; modification-token wrap
after 2^31 changes and evaded invalid-iterator valuation are not exercised;
load nonnumeric/integer failed-getter and unrepresentable float cases are
excluded (#67). The cargo fixture initializes the scalar collector without
world validation; failed station/cargo/company policy and missing HasData
probes are unexecuted, while their validation and goods-guard bodies and
order are checked unchanged.
"""

from pathlib import Path

import probe

ROOT = probe.ROOT
REFERENCE = probe.REFERENCE


def main():
    out = probe.output("script-list-comparison")
    squirrel = (REFERENCE / "src/script/squirrel.cpp").read_text()
    allocator_body = probe.body(squirrel, "struct ScriptAllocator {") + ";"
    if allocator_body not in (ROOT / "src/script/squirrel.cpp").read_text():
        raise RuntimeError("Original VM allocator changed")
    allocator = out / "allocator.cpp"
    allocator.write_text(
        '#include "stdafx.h"\n#include "3rdparty/fmt/format.h"\n#include "error_func.h"\n'
        '#include "script/script_fatalerror.hpp"\n#include <squirrel.h>\n'
        "static struct { struct { uint32_t script_max_memory_megabytes = 1; } script; } _settings_game;\n"
        + allocator_body
        + "\nScriptAllocator *_squirrel_allocator = nullptr;\n"
        "void *sq_vm_malloc(SQUnsignedInteger n) { return _squirrel_allocator->Malloc(n); }\n"
        "void *sq_vm_realloc(void *p, SQUnsignedInteger a, SQUnsignedInteger b) { return _squirrel_allocator->Realloc(p,a,b); }\n"
        "void sq_vm_free(void *p, SQUnsignedInteger n) { _squirrel_allocator->Free(p,n); }\n"
        "void BeginAllocator() { _squirrel_allocator = new ScriptAllocator; }\n"
        "size_t EndAllocator() { auto n = _squirrel_allocator->GetAllocatedSize(); delete _squirrel_allocator; _squirrel_allocator = nullptr; return n; }\n"
    )
    tile_source = (REFERENCE / "src/script/api/script_tilelist.cpp").read_text()
    signatures = (
        "bool ScriptTileList::SaveObject(",
        "ScriptObject *ScriptTileList::CloneObject(",
    )
    bodies = [probe.body(tile_source, signature) for signature in signatures]
    if any(
        body not in (ROOT / "src/script/api/script_tilelist.cpp").read_text()
        for body in bodies
    ):
        raise RuntimeError("TileList persistence/clone changed")
    tile = out / "tile-persistence.cpp"
    tile.write_text(
        '#include "stdafx.h"\n#include "script/api/script_tilelist.hpp"\n'
        + "\n".join(bodies)
        + "\n"
    )
    # Scalar cargo gaps use exact original Update/SetValue/destructor and planned
    # traversal bodies. Only direct collector construction omits world lookup;
    # saved-game regressions remain the actual station/cargo validity evidence.
    station_source = (REFERENCE / "src/script/api/script_stationlist.cpp").read_text()
    candidate_station = (ROOT / "src/script/api/script_stationlist.cpp").read_text()
    validation = probe.body(station_source, "CargoCollector::CargoCollector(")
    validation_body = validation[
        validation.index("{") + 1 : validation.rfind("}")
    ].strip()
    active_station = candidate_station
    if validation_body.replace("\n\t", "\n\t\t") not in active_station:
        raise RuntimeError("Station-before-cargo validation body/order changed")
    guards = "if (collector.GE() == nullptr) return;\n\tif (!collector.GE()->HasData()) return;"
    if guards not in active_station or guards not in station_source:
        raise RuntimeError("Null goods / HasData guard order changed")
    cargo_signatures = (
        "class CargoCollector {",
        "CargoCollector::~CargoCollector()",
        "void CargoCollector::SetValue()",
        "void CargoCollector::Update(",
    )
    cargo_bodies = {
        signature: probe.body(station_source, signature)
        for signature in cargo_signatures
    }
    initializer = station_source[
        station_source.index("CargoCollector::CargoCollector(") :
    ]
    initializer = initializer[: initializer.index("{")]
    cargo_text = "\n".join(
        body + (";" if name == "class CargoCollector {" else "")
        for name, body in cargo_bodies.items()
    )
    cargo_text = cargo_text.replace(
        cargo_bodies["void CargoCollector::Update("],
        "template <ScriptStationList_Cargo::CargoSelector Tselector>\n"
        + cargo_bodies["void CargoCollector::Update("],
    )
    cargo_text += "\n" + initializer + "{ /* Direct fixture: no world access. */ }\n"
    planned = probe.body(station_source, "void ScriptStationList_CargoPlanned::Add(")
    planned_loop = planned[
        planned.index("FlowStatMap::const_iterator iter") : planned.rfind("}")
    ]
    planned_loop = planned_loop.replace("collector.GE()->GetData().flows", "flows")
    cargo_text += (
        "\ntemplate <ScriptStationList_Cargo::CargoSelector Tselector>\nvoid OriginalCargoPlanned(CargoCollector &collector, const FlowStatMap &flows) {\n"
        + planned_loop
        + "}\n"
    )
    filtered = probe.body(
        station_source,
        "ScriptStationList_CargoPlannedFromByVia::ScriptStationList_CargoPlannedFromByVia(",
    )
    filtered_loop = filtered[
        filtered.index("FlowStatMap::const_iterator iter") : filtered.rfind("}")
    ]
    filtered_loop = filtered_loop.replace("collector.GE()->GetData().flows", "flows")
    cargo_text += (
        "\nvoid OriginalCargoFind(CargoCollector &collector, const FlowStatMap &flows, StationID from) {\nconstexpr auto CS_FROM_BY_VIA = ScriptStationList_Cargo::CS_FROM_BY_VIA;\n"
        + filtered_loop
        + "}\n"
    )
    cargo_text = cargo_text.replace("CargoCollector", "ReferenceCargoCollector")
    # Reader type deduction accepts both native shares and the Rust borrowing
    # facade; the copied reference traversal and reduction expressions stay intact.
    cargo_text = cargo_text.replace(
        "const FlowStat::SharesMap *shares", "const auto *shares"
    )
    cargo_text = cargo_text.replace(
        "FlowStat::SharesMap::const_iterator flow_iter", "auto flow_iter"
    )
    (out / "cargo-reducer.hpp").write_text(cargo_text)
    vm_sources = sorted((REFERENCE / "src/3rdparty/squirrel/squirrel").glob("*.cpp"))
    for source in vm_sources:
        if source.read_bytes() != (ROOT / source.relative_to(REFERENCE)).read_bytes():
            raise RuntimeError("Bundled VM source changed")

    def compile(command, name):
        probe.compile(command, out / f"{name}-compile.log")

    # The inherited export parser tracks access markers without recognizing an
    # un-inherited nested helper class. Preserve the actual List API in all modes.
    exporter = Path("cmake/scripts/SquirrelExport.cmake")
    if (ROOT / exporter).read_bytes() != (REFERENCE / exporter).read_bytes():
        raise RuntimeError("Original binding generator changed")

    for apilc, apiuc in (("ai", "AI"), ("game", "GS"), ("template", "Template")):
        generated = []
        for label, source in (("reference", REFERENCE), ("candidate", ROOT)):
            target = out / f"bindings-{label}-{apilc}.sq.hpp"
            compile(
                [
                    "cmake",
                    f"-DSCRIPT_API_SOURCE_FILE={source / 'src/script/api/squirrel_export.sq.hpp.in'}",
                    f"-DSCRIPT_API_BINARY_FILE={target}",
                    f"-DSCRIPT_API_FILE={source / 'src/script/api/script_list.hpp'}",
                    f"-DAPIUC={apiuc}",
                    f"-DAPILC={apilc}",
                    "-P",
                    str(source / exporter),
                ],
                f"bindings-{label}-{apilc}",
            )
            generated.append(target.read_bytes())
        if generated[0] != generated[1]:
            raise RuntimeError(f"ScriptList {apilc} generated bindings changed")

    # VM and allocator are real unchanged code. Each label uses its actual string helpers.
    flags = [
        probe.compiler(),
        "-std=c++20",
        "-DFMT_HEADER_ONLY",
        "-DUNIX",
        "-DPOINTER_IS_64BIT",
        "-Wno-multichar",
        "-ffunction-sections",
        "-fdata-sections",
        "-I",
        str(out),
        "-I",
        str(ROOT / "build-rust/generated"),
    ]
    objects = {}
    for group, source_root in (("reference", REFERENCE), ("rust", ROOT)):
        objects[group] = []
        group_flags = flags + (["-DWITH_RUST"] if group == "rust" else [])
        for number, source in enumerate(vm_sources + [allocator]):
            # Relative includes resolve beside the source; use this group's copy.
            if source.is_relative_to(REFERENCE):
                source = source_root / source.relative_to(REFERENCE)
            obj = out / f"vm-{group}-{number}.o"
            compile(
                [
                    *group_flags,
                    "-O2",
                    "-I",
                    str(source_root / "src"),
                    "-I",
                    str(source_root / "src/3rdparty/squirrel/include"),
                    "-c",
                    str(source),
                    "-o",
                    str(obj),
                ],
                f"vm-{group}-{number}",
            )
            objects[group].append(obj)
    fixture = ROOT / "tools/migration/script-list-comparison.cpp"
    for mode in ("O0", "O2"):

        def command(label, source, mode=mode):
            return [
                *flags,
                "-" + mode,
                "-I",
                source / "src",
                "-I",
                source / "src/3rdparty/squirrel/include",
                fixture,
                tile,
                source / "src/script/api/script_list.cpp",
                source / "src/core/string_consumer.cpp",
                source / "src/core/utf8.cpp",
                *objects["rust" if label == "candidate" else "reference"],
                "-Wl,--gc-sections",
            ]

        output = probe.compare(out, mode, command)
    probe.passed("ScriptList comparison", records_at_O0_and_O2=len(output.splitlines()))


if __name__ == "__main__":
    main()
