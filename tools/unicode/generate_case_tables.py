"""Generate or check the case tables from the Unicode 14.0.0 UCD.

Pass a directory containing UnicodeData.txt and SpecialCasing.txt from
https://www.unicode.org/Public/14.0.0/ucd/.
Run with --check to compare the committed tables with the generated text.
"""

import argparse
import hashlib
from pathlib import Path

VERSION = "14.0.0"
UCD_SHA256 = {
    "UnicodeData.txt": "36018e68657fdcb3485f636630ffe8c8532e01c977703d2803f5b89d6c5feafb",
    "SpecialCasing.txt": "c667b45908fd269af25fd55d2fc5bbc157fb1b77675936e25c513ce32e080334",
}
OUTPUT = Path(__file__).resolve().parents[2] / "crates/interp/src/case_tables.rs"


def records(path):
    for line in path.read_text(encoding="utf-8").splitlines():
        record = line.split("#", 1)[0].strip()
        if record:
            yield [field.strip() for field in record.split(";")]


def mapping_ranges(mapping):
    """Group mappings with the same offset and a stride of one or two."""
    entries = sorted(mapping.items())
    i = 0
    while i < len(entries):
        start, target = entries[i]
        offset = target - start
        stride = 1
        if i + 1 < len(entries) and entries[i + 1][0] == start + 2:
            stride = 2
        end = start
        i += 1
        while i < len(entries) and entries[i] == (end + stride, end + stride + offset):
            end += stride
            i += 1
        yield start, end, stride, offset


def category_ranges(path):
    """General categories of assigned code points, with First/Last pairs
    expanded and adjacent code points of one category merged."""
    ranges = []
    first = None
    for code, name, category, *_ in records(path):
        code = int(code, 16)
        if name.endswith(", First>"):
            first = code
            continue
        start = first if name.endswith(", Last>") else code
        first = None
        if ranges and ranges[-1][2] == category and ranges[-1][1] + 1 == start:
            ranges[-1][1] = code
        else:
            ranges.append([start, code, category])
    return ranges


def render(ucd):
    for name, expected in UCD_SHA256.items():
        actual = hashlib.sha256((ucd / name).read_bytes()).hexdigest()
        if actual != expected:
            raise ValueError(f"Expected the official Unicode {VERSION} {name}")

    upper, lower = {}, {}
    for code, *_, uppercase, lowercase, _titlecase in records(ucd / "UnicodeData.txt"):
        code = int(code, 16)
        if uppercase:
            upper[code] = int(uppercase, 16)
        if lowercase:
            lower[code] = int(lowercase, 16)

    expansions = {}
    for code, lowercase, _titlecase, uppercase, condition, *_ in records(ucd / "SpecialCasing.txt"):
        if condition:
            continue
        code = int(code, 16)
        uppercase = [int(part, 16) for part in uppercase.split()]
        lowercase = [int(part, 16) for part in lowercase.split()]
        if uppercase != [upper.get(code, code)]:
            expansions[code] = uppercase
        if lowercase != [lower.get(code, code)] and (code, lowercase) != (0x130, [0x69, 0x307]):
            raise ValueError("Update change_case for the new unconditional lowercase mapping")

    lines = [
        f"//! Generated Unicode {VERSION} case data for the B4J recording profile.",
        "//! Regenerate with tools/unicode/generate_case_tables.py.",
        f"//! Source: https://www.unicode.org/Public/{VERSION}/ucd/.",
        "//! Copyright © 1991-2022 Unicode, Inc. See THIRD-PARTY-NOTICES.txt.",
        "",
        "use super::{Gc, Range};",
        "",
    ]
    for name, mapping in (("UPPER", upper), ("LOWER", lower)):
        lines.append(f"pub(super) const {name}: &[Range] = &[")
        for start, end, stride, offset in mapping_ranges(mapping):
            lines.append(f"    (0x{start:04X}, 0x{end:04X}, {stride}, {offset}),")
        lines.extend(("];", ""))

    lines.append("pub(super) const UPPER_EXPANSIONS: &[(char, &str)] = &[")
    for code, mapping in sorted(expansions.items()):
        key = "\\u{%X}" % code
        value = "".join("\\u{%X}" % part for part in mapping)
        lines.append(f"    ('{key}', \"{value}\"),")
    lines.extend(("];", ""))

    lines.append("/// Unassigned code points (`Cn`) are left out.")
    lines.append("pub(super) const GENERAL_CATEGORIES: &[(u32, u32, Gc)] = &[")
    for start, end, category in category_ranges(ucd / "UnicodeData.txt"):
        lines.append(f"    (0x{start:04X}, 0x{end:04X}, Gc::{category}),")
    lines.extend(("];", ""))
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("ucd", type=Path, help="directory containing the Unicode data files")
    parser.add_argument("--check", action="store_true", help="verify without writing")
    args = parser.parse_args()
    text = render(args.ucd)
    if args.check:
        if OUTPUT.read_text(encoding="utf-8") != text:
            raise SystemExit(f"{OUTPUT} differs from the generated tables")
    else:
        OUTPUT.write_text(text, encoding="utf-8")


if __name__ == "__main__":
    main()
