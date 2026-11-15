"""Generate or check the case tables from the Unicode 14.0.0 UCD.

Pass a directory containing UnicodeData.txt, SpecialCasing.txt and
DerivedCoreProperties.txt from https://www.unicode.org/Public/14.0.0/ucd/.
Run with --check to compare the committed tables with the generated text.
"""

import argparse
import hashlib
from pathlib import Path

VERSION = "14.0.0"
UCD_SHA256 = {
    "UnicodeData.txt": "36018e68657fdcb3485f636630ffe8c8532e01c977703d2803f5b89d6c5feafb",
    "SpecialCasing.txt": "c667b45908fd269af25fd55d2fc5bbc157fb1b77675936e25c513ce32e080334",
    "DerivedCoreProperties.txt": "e3eddd7d469cd1b0feed7528defad1a1cc7c6a9ceb0ae4446a6d10921ed2e7bc",
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


def property_ranges(records, property_name):
    ranges = []
    for span, name in records:
        if name == property_name:
            start, _, end = span.partition("..")
            ranges.append((int(start, 16), int(end or start, 16)))
    if not ranges:
        raise ValueError(f"Missing Unicode property {property_name}")
    merged = []
    for start, end in sorted(ranges):
        if merged and start <= merged[-1][1] + 1:
            previous_start, previous_end = merged[-1]
            merged[-1] = previous_start, max(previous_end, end)
        else:
            merged.append((start, end))
    return merged


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
        "use super::Range;",
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

    properties = list(records(ucd / "DerivedCoreProperties.txt"))
    for name, constant in (("Cased", "CASED"), ("Case_Ignorable", "CASE_IGNORABLE")):
        lines.append(f"pub(super) const {constant}: &[(u32, u32)] = &[")
        for start, end in property_ranges(properties, name):
            lines.append(f"    (0x{start:04X}, 0x{end:04X}),")
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
