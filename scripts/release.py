#!/usr/bin/env python3
"""Release helper: lockstep versions and license files for every openhub-bo package.

    python3 scripts/release.py check [--tag v0.2.0]   # CI and release: everything consistent?
    python3 scripts/release.py set-version 0.2.0      # bump every manifest
    python3 scripts/release.py sync                   # copy LICENSE/NOTICE into each package

Every package (Rust crates, PyPI, npm, RubyGems) shares one version. Internal
dependency ranges allow any release of the same minor series (">=0.2.0,<0.3.0",
"~> 0.2.0"), so a patch release of one package keeps working with the others.
Standard library only.
"""

from __future__ import annotations

import argparse
import re
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PKGS = ["core", "qr", "fx", "accounts", "payouts"]
LEGAL = ["LICENSE", "NOTICE"]
SEMVER = re.compile(r"^\d+\.\d+\.\d+$")

# Directories that publish a package and must ship LICENSE/NOTICE.
PACKAGE_DIRS = [
    *(f"bindings/python/{p}" for p in [*PKGS, "meta"]),
    *(f"bindings/typescript/packages/{p}" for p in PKGS),
    *(f"bindings/ruby/openhub-bo-{p}" for p in PKGS),
    "bindings/ruby/openhub-bo",
]


def next_minor(version: str) -> str:
    major, minor, _ = (int(x) for x in version.split("."))
    return f"{major + 1}.0.0" if major >= 1 else f"0.{minor + 1}.0"


def rules(version: str) -> list[tuple[str, re.Pattern[str], str]]:
    """(file, pattern with one group around the version text, replacement)."""
    py_range = f">={version},<{next_minor(version)}"
    rb_range = f"~> {version}"
    out: list[tuple[str, re.Pattern[str], str]] = []

    def add(path: str, pattern: str, repl: str) -> None:
        out.append((path, re.compile(pattern, re.M), repl))

    for ws in ["Cargo.toml", "bindings/typescript/Cargo.toml"]:
        add(ws, r'^\[workspace\.package\]\nversion = "([^"]+)"', version)
    add("bindings/python/meta/pyproject.toml", r'^version = "([^"]+)"', version)
    for p in [*PKGS[1:], "meta"]:
        add(f"bindings/python/{p}/pyproject.toml", r'"openhub-bo-core([^"]+)"', py_range)
    for p in PKGS[1:]:
        add("bindings/python/meta/pyproject.toml", rf'"openhub-bo-{p}([^"]+)"', py_range)
    for p in PKGS:
        add(f"bindings/typescript/packages/{p}/package.json", r'^  "version": "([^"]+)"', version)
        add(f"bindings/ruby/openhub-bo-{p}/openhub-bo-{p}.gemspec", r'spec\.version = "([^"]+)"', version)
        add(f"bindings/ruby/openhub-bo-{p}/ext/openhub_bo_{p}/Cargo.toml", r'^version = "([^"]+)"', version)
        if p != "core":
            add(f"bindings/ruby/openhub-bo-{p}/openhub-bo-{p}.gemspec",
                r'add_dependency "openhub-bo-core", "([^"]+)"', rb_range)
    add("bindings/ruby/openhub-bo/openhub-bo.gemspec", r'spec\.version = "([^"]+)"', version)
    add("bindings/ruby/openhub-bo/openhub-bo.gemspec", r'add_dependency "openhub-bo-#\{pkg\}", "([^"]+)"', rb_range)
    add("bindings/ruby/openhub-bo-core/lib/openhub_bo/core/version.rb", r'VERSION = "([^"]+)"', version)
    return out


def current_version() -> str:
    text = (ROOT / "Cargo.toml").read_text()
    m = re.search(r'^\[workspace\.package\]\nversion = "([^"]+)"', text, re.M)
    if not m:
        sys.exit("Cargo.toml: [workspace.package] version not found")
    return m.group(1)


def check(tag: str | None) -> int:
    version = current_version()
    errors: list[str] = []
    if tag is not None and tag.removeprefix("v") != version:
        errors.append(f"tag {tag} does not match version {version}")
    for path, pattern, expected in rules(version):
        text = (ROOT / path).read_text()
        found = [m.group(1) for m in pattern.finditer(text)]
        if not found:
            errors.append(f"{path}: pattern {pattern.pattern!r} not found")
        errors += [f"{path}: {value!r}, expected {expected!r}" for value in found if value != expected]
    for directory in PACKAGE_DIRS:
        for name in LEGAL:
            copy = ROOT / directory / name
            if not copy.exists() or copy.read_bytes() != (ROOT / name).read_bytes():
                errors.append(f"{directory}/{name} missing or outdated (run: scripts/release.py sync)")
    for error in errors:
        print(f"error: {error}", file=sys.stderr)
    if not errors:
        print(f"ok: every package at {version}, license files in sync")
    return 1 if errors else 0


def set_version(version: str) -> int:
    if not SEMVER.match(version):
        sys.exit(f"not a MAJOR.MINOR.PATCH version: {version}")
    for path, pattern, value in rules(version):
        file = ROOT / path
        text, count = pattern.subn(lambda m: m.group(0).replace(m.group(1), value), file.read_text())
        if not count:
            sys.exit(f"{path}: pattern {pattern.pattern!r} not found")
        file.write_text(text)
    print(f"set every package to {version}; update Cargo.lock files with `cargo update -w`")
    return check(None)


def sync() -> int:
    for directory in PACKAGE_DIRS:
        for name in LEGAL:
            shutil.copyfile(ROOT / name, ROOT / directory / name)
    print(f"copied {', '.join(LEGAL)} into {len(PACKAGE_DIRS)} packages")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("check").add_argument("--tag")
    sub.add_parser("set-version").add_argument("version")
    sub.add_parser("sync")
    args = parser.parse_args()
    if args.command == "check":
        return check(args.tag)
    if args.command == "set-version":
        return set_version(args.version)
    return sync()


if __name__ == "__main__":
    sys.exit(main())
