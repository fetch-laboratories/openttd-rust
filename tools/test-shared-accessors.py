#!/usr/bin/env python3
"""Run the real #249 CLI for every seeded category, upstream scope and ratchet."""

import re
import subprocess
import tempfile
from pathlib import Path

SCRIPT = Path(__file__).with_name("shared-accessors.py")
EXPRESSIONS = (
    "v->owner|s->index.base()|b->owner|c->money|_settings_game.vehicle.plane_speed|"
    "_settings_client.sound.disaster|TimerGameCalendar::date|TimerGameEconomy::date|"
    "TimerGameTick::counter|_game_mode == GM_EDITOR|_current_company.base()|Map::Size()|"
    "Canonical(v)|to_underlying(Canonical(v))|static_cast<Vehicle *>(context)->owner|"
    "Company::Get(id)->money"
).split("|")
with tempfile.TemporaryDirectory() as directory:
    root = Path(directory)
    for name in ["src/rust", "tools", "migration"]:
        (root / name).mkdir(parents=True)

    def git(*args):
        return subprocess.check_output(["git", *args], cwd=root, text=True).strip()

    git("init", "-q")
    git("config", "user.name", "Accessor regression")
    git("config", "user.email", "accessors@example.invalid")
    port = root / "src/port.cpp"
    original = "int Inherited(Vehicle *v) { return v->owner; }"
    port.write_text(original)
    git("add", ".")
    git("commit", "-qm", "Add: Reference fixture")
    (root / "migration/baseline.json").write_text(
        '{"commit":"%s"}' % git("rev-parse", "HEAD")
    )
    (root / "tools/shared-accessors.py").write_text(SCRIPT.read_text())
    canonical = "int Canonical(Vehicle *v) { return v->owner; }"
    for shared in ["services.h", "engine_services.h", "presentation_services.h"]:
        (root / "src/rust" / shared).write_text(canonical)
    baseline = root / "tools/shared-accessor-baseline.txt"
    baseline.write_text("")
    git("add", ".")
    git("commit", "-qm", "Add: Empty duplicate baseline")
    base = git("rev-parse", "HEAD")

    def check():
        return subprocess.run(
            ["python3", "tools/shared-accessors.py", "--base-ref", base],
            cwd=root,
            capture_output=True,
            text=True,
        )

    assert check().returncode == 0
    for expression in EXPRESSIONS:
        for prefix in ["int Renamed", "[]"]:
            port.write_text(
                original
                + f"{prefix}(const Vehicle *const v, Station *s, BaseStation *b, Company *c) noexcept {{ return {expression}; }}"
            )
            if prefix == "[]" and expression == "Map::Size()":
                port.write_text(original + "[] { return Map::Size(); }")
            result = check()
            assert result.returncode == 1 and "new shared accessor:" in result.stdout
    fingerprint = result.stdout.split("new shared accessor: ")[1].splitlines()[0]
    baseline.write_text(fingerprint + "\n")
    assert "addition forbidden" in check().stdout and check().returncode == 1
    git("commit", "-qam", "Change: Grandfather fixture fingerprint")
    base = git("rev-parse", "HEAD")
    assert check().returncode == 0
    copy = port.read_text()[len(original) :]
    for changed in [original + copy * 2, original + copy.replace("money", "owner")]:
        port.write_text(changed)
        assert "new shared accessor:" in check().stdout and check().returncode == 1
    port.write_text(original)
    assert "remove retired" in check().stdout
    baseline.write_text("")
    assert check().returncode == 0
    owner = root / "src/owner.rs"
    owner.write_text('#[unsafe(no_mangle)] pub extern "C" fn openttd_rust_dead() {}')
    caller = root / "tools/caller.py"
    caller.write_text("")
    include_caller = root / "tools/caller.inc"
    include_caller.write_text("")
    git("add", ".")
    assert "orphan | openttd_rust_dead" in check().stdout
    for declaration in ["void", 'extern "C" void']:
        port.write_text(original + f"\n{declaration} openttd_rust_dead();\n")
        assert "orphan | openttd_rust_dead" in check().stdout
    for target, call, orphan in [
        (port, "void Call() { openttd_rust_dead(); }", False),
        (owner, "fn call() { openttd_rust_dead(); }", False),
        (caller, "source = 'openttd_rust_dead();'", False),
        (include_caller, "void FixtureCall() { openttd_rust_dead(); }", False),
        (include_caller, "// openttd_rust_dead()", True),
        (port, "void Call() {\n    openttd_rust_dead();\n}", False),
        (port, "void Call() {\n    openttd_rust_dead(\n        1);\n}", False),
        (port, "// openttd_rust_dead\n/* openttd_rust_dead */", True),
        (owner, "// openttd_rust_dead", True),
        (caller, "x = 1  # openttd_rust_dead()", True),
    ]:
        before = target.read_text()
        target.write_text(before + "\n" + call + "\n")
        result = check()
        assert ("orphan |" in result.stdout) == orphan == bool(result.returncode)
        target.write_text(before)
    caller.write_text("source = 'openttd_rust_dead();'")
    shared_source = root / "src/rust/services.h"
    shared_source.write_text(canonical + "\nvoid SharedCallback() noexcept {}\n")
    shared_bindings = (
        "\nOpenTTDFixtureServices shared_one{.a = SharedCallback,\n};\n"
        "OpenTTDFixtureServices shared_two{.b = SharedCallback,\n};\n"
    )
    port.write_text(port.read_text() + shared_bindings)
    assert check().returncode == 0, check().stdout
    before = port.read_text()
    port.write_text(before + "\nstatic void SharedCallback() noexcept {}\n")
    assert "binding | SharedCallback | 2" in check().stdout
    port.write_text(before)
    duplicate = "\nvoid Callback() noexcept {}\nOpenTTDFixtureServices one{.a = Callback,\n};\nOpenTTDFixtureServices two{.b = Callback,\n};\n"
    write = "\nvoid Write(uint8_t value) { _current_company = value; }\n"
    port.write_text(port.read_text() + duplicate + write)
    result = check()
    assert "binding | Callback | 2" in result.stdout and "write |" in result.stdout
    fingerprints = re.findall(r"new shared accessor: (.*)", result.stdout)
    baseline.write_text("\n".join(fingerprints) + "\n")
    assert "addition forbidden" in check().stdout
    baseline.write_text("")
    extra = "\nOpenTTDFixtureServices extra{.c = Callback,\n};\n"
    port.write_text(port.read_text() + extra)
    git("commit", "-qam", "Add: Actual boundary seed source")
    base = git("rev-parse", "HEAD")
    port.write_text(port.read_text().removesuffix(extra))
    baseline.write_text("\n".join(fingerprints) + "\n")
    assert check().returncode == 0, check().stdout
    baseline.write_text(baseline.read_text() + "binding | Invented | 2\n")
    assert "addition forbidden" in check().stdout
    baseline.write_text("\n".join(fingerprints) + "\n")
    git("commit", "-qam", "Add: Boundary baseline")
    base = git("rev-parse", "HEAD")
    port.write_text(port.read_text() + extra)
    result = check()
    assert "new shared accessor:" in result.stdout
    added = result.stdout.split("new shared accessor: ")[1].splitlines()[0]
    baseline.write_text(baseline.read_text() + added + "\n")
    assert "addition forbidden" in check().stdout
    port.write_text(original)
    owner.write_text("")
    assert "remove retired" in check().stdout
    baseline.write_text("")
    assert check().returncode == 0
    seed = "\nint First(Vehicle *v) noexcept { return NativeLeaf(v, EVENT_A); }\nOpenTTDFixtureServices first{.a = First,\n};\n"
    port.write_text(original + seed)
    git("commit", "-qam", "Add: Single native callee fixture")
    base = git("rev-parse", "HEAD")
    call = "return NativeLeaf(v, EVENT_A);"
    variants = [
        (call, "noexcept", True),
        (
            "Vehicle *p = static_cast<Vehicle *>(v.shell); "
            "return NativeLeaf(p, EVENT_A);",
            "noexcept",
            True,
        ),
        (call.replace("EVENT_A", "EVENT_B"), "noexcept", False),
        ("Notify(v); " + call, "noexcept", False),
        (call.replace("(v,", "(v->owner + v->index,"), "noexcept", False),
        (call, "", False),
    ]
    for body, qualifier, copied in variants:
        for prefix in ["int Second", "[]"]:
            definition = f"{prefix}(Vehicle *v) {qualifier} {{ {body} }}"
            binding = "Second" if prefix != "[]" else definition
            source = definition if prefix != "[]" else ""
            port.write_text(
                original
                + seed
                + source
                + f"\nOpenTTDFixtureServices second{{.b = {binding},\n}};\n"
            )
            result = check()
            assert ("callee | NativeLeaf(arg0,EVENT_A)" in result.stdout) == copied
            assert bool(result.returncode) == copied, result.stdout
    for kind, copied in [("auto", True), ("uint8_t", False)]:
        port.write_text(
            original
            + seed.replace("Vehicle *v", "Vehicle *v, uint32_t mode").replace(
                "EVENT_A", "mode"
            )
            + f"int Second(Vehicle *v, uint32_t mode) noexcept {{ {kind} value = mode; "
            "return NativeLeaf(v, value); }\nOpenTTDFixtureServices second{.b = Second,\n};\n"
        )
        result = check()
        assert ("callee |" in result.stdout) == copied, result.stdout
        assert bool(result.returncode) == copied, result.stdout
    port.write_text(
        original
        + seed
        + "int Second(Vehicle *v) noexcept { "
        + call
        + " }\nOpenTTDFixtureServices second{.b = Second,\n};\n"
    )
    git("commit", "-qam", "Add: Existing callee copies fixture")
    base = git("rev-parse", "HEAD")
    assert check().returncode == 0, check().stdout
    port.write_text(
        port.read_text()
        + "int Third(Vehicle *v) noexcept { return NativeLeaf(v, EVENT_A); }\nOpenTTDFixtureServices third{.c = Third,\n};\n"
    )
    result = check()
    assert "callee |" in result.stdout and result.returncode == 1, result.stdout
    fingerprint = result.stdout.split("new shared accessor: ")[1].splitlines()[0]
    baseline.write_text(fingerprint + "\n")
    assert "addition forbidden" in check().stdout
    port.write_text(original + seed)
    baseline.write_text("")
    assert check().returncode == 0, check().stdout
    baseline.unlink()
    assert check().returncode != 0
