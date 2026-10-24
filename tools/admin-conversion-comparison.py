#!/usr/bin/env python3
"""Compare scoped Admin conversion gaps using unchanged pinned bodies and the real VM.

Limits: native Linux; the logger records entry text and state rather than
game-log storage; extra control allocations have different failure timing; the
unchanged ScriptAllocator reads a fixture-only script_max_memory_megabytes of 1.
"""

import probe

ROOT = probe.ROOT
REFERENCE = probe.REFERENCE
extract = probe.body


def main():
    out = probe.output("admin-conversion-comparison")
    admin = (REFERENCE / "src/script/api/script_admin.cpp").read_text()
    event = (REFERENCE / "src/script/api/script_event_types.cpp").read_text()
    squirrel = (REFERENCE / "src/script/squirrel.cpp").read_text()
    bodies = {
        "outgoing": extract(admin, "bool ScriptAdminMakeJSON("),
        "incoming": extract(event, "static bool ScriptEventAdminPortReadValue("),
        "get_object": extract(event, "SQInteger ScriptEventAdminPort::GetObject("),
        "allocator": extract(squirrel, "struct ScriptAllocator {") + ";",
    }
    original = out / "original.cpp"
    original.write_text(
        '#include "stdafx.h"\n#include "script/admin_conversion.hpp"\n'
        '#include "script/api/script_log.hpp"\n#include "script/script_instance.hpp"\n'
        + bodies["outgoing"].replace("ScriptAdminMakeJSON", "OriginalOutgoing")
        + "\n"
        + bodies["incoming"].replace(
            "ScriptEventAdminPortReadValue", "OriginalReadValue"
        )
        + "\n"
        + bodies["get_object"]
        .replace(
            "SQInteger ScriptEventAdminPort::GetObject(HSQUIRRELVM vm)",
            "SQInteger OriginalGetObject(HSQUIRRELVM vm, const std::string &raw)",
        )
        .replace("this->json", "raw")
        .replace("ScriptEventAdminPortReadValue", "OriginalReadValue")
        + "\n"
    )
    direct = bodies["get_object"].replace(
        "SQInteger ScriptEventAdminPort::GetObject(HSQUIRRELVM vm)",
        "SQInteger OriginalFromJSON(HSQUIRRELVM vm, nlohmann::json &json)",
    )
    direct = direct.replace(
        "\tauto json = nlohmann::json::parse(this->json, nullptr, false);", ""
    )
    direct = direct.replace("ScriptEventAdminPortReadValue", "OriginalReadValue")
    with original.open("a") as output:
        output.write(direct + "\n")
    allocator = out / "allocator.cpp"
    allocator.write_text(
        '#include "stdafx.h"\n#include "3rdparty/fmt/format.h"\n#include <cstdint>\n'
        '#include "error_func.h"\n#include "script/script_fatalerror.hpp"\n#include <squirrel.h>\n'
        + "static struct { struct { uint32_t script_max_memory_megabytes = 1; } script; } _settings_game;\n"
        + bodies["allocator"]
        + "\nScriptAllocator *_squirrel_allocator = nullptr;\n"
        + "void *sq_vm_malloc(SQUnsignedInteger n) { return _squirrel_allocator->Malloc(n); }\n"
        + "void *sq_vm_realloc(void *p, SQUnsignedInteger a, SQUnsignedInteger b) { return _squirrel_allocator->Realloc(p,a,b); }\n"
        + "void sq_vm_free(void *p, SQUnsignedInteger n) { _squirrel_allocator->Free(p,n); }\n"
        + "void BeginAllocator() { _settings_game.script.script_max_memory_megabytes = 1; _squirrel_allocator = new ScriptAllocator; }\n"
        + "size_t EndAllocator() { auto n = _squirrel_allocator->GetAllocatedSize(); delete _squirrel_allocator; _squirrel_allocator = nullptr; return n; }\n"
    )
    if probe.migration.rust_configuration(probe.BUILD)["platform"] != "Linux":
        raise RuntimeError(
            "GNU link wrapping in this exception/owner fixture currently requires native Linux"
        )
    flags = [
        probe.compiler(),
        "-std=c++20",
        "-DFMT_HEADER_ONLY",
        "-DWITH_RUST",
        "-DUNIX",
        "-ffunction-sections",
        "-fdata-sections",
        "-I",
        str(ROOT / "src"),
        "-I",
        str(ROOT / "src/3rdparty/squirrel/include"),
        "-I",
        str(ROOT / "src/script/api"),
        "-I",
        str(ROOT / "build-rust/generated"),
        "-I",
        str(ROOT / "build-rust/generated/script/api"),
        *probe.target_flags(),
    ]

    def compile_run(command, label):
        probe.compile(command, out / f"{label}.log")

    # One serial compiler, retaining the actual bundled VM. Its sources are unchanged.
    common_sources = sorted((ROOT / "src/3rdparty/squirrel/squirrel").glob("*.cpp"))
    vm_sources = sorted((ROOT / "src/3rdparty/squirrel").rglob("*.cpp")) + sorted(
        (ROOT / "src/3rdparty/squirrel").rglob("*.h")
    )
    for source in vm_sources:
        if source.read_bytes() != (REFERENCE / source.relative_to(ROOT)).read_bytes():
            raise RuntimeError(f"Bundled VM source changed: {source}")
    common_sources += [
        ROOT / "src/core/string_consumer.cpp",
        ROOT / "src/core/utf8.cpp",
        allocator,
    ]
    objects = []
    for number, source in enumerate(common_sources):
        obj = out / f"vm-{number}.o"
        compile_run(
            [*flags, "-O2", "-c", str(source), "-o", str(obj)], f"vm-{number}-compile"
        )
        objects.append(obj)
    counts = {}
    for mode in ("O0", "O2"):
        executable = out / f"comparison-{mode}"
        sources = [
            ROOT / "tools/migration/admin-conversion-comparison.cpp",
            original,
            ROOT / "src/script/admin_conversion.cpp",
            ROOT / "src/script/api/script_admin.cpp",
            ROOT / "src/script/api/script_event_types.cpp",
        ]
        command = [
            *flags,
            "-" + mode,
            *map(str, sources),
            *map(str, objects),
            *probe.rust(define=False),
            "-Wl,--gc-sections",
            "-Wl,--wrap=openttd_rust_admin_conversion_create",
            "-Wl,--wrap=openttd_rust_admin_conversion_destroy",
            "-o",
            str(executable),
        ]
        compile_run(command, f"{mode}-compile")
        # Each process compares original and candidate on the same live VM.
        # Instance-key hashes depend on allocation addresses: separate processes may
        # legitimately visit stringification collisions in a different order.
        counts[f"{mode}_records"] = len(
            probe.execute([executable], out / mode).splitlines()
        )
    probe.passed("Admin conversion comparison", **counts)


if __name__ == "__main__":
    main()
