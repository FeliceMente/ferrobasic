//! Java `String` operations on UTF-16 code units.

use std::cmp::Ordering;

/// Text as UTF-16 code units, as a Rust string: an unpaired surrogate
/// becomes `?`, as Java's encoders write it.
pub fn to_rust(s: &[u16]) -> String {
    char::decode_utf16(s.iter().copied())
        .map(|c| c.unwrap_or('?'))
        .collect()
}

pub fn to_utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// `String.compareTo`.
pub fn compare_to(a: &[u16], b: &[u16]) -> i32 {
    for (x, y) in a.iter().zip(b) {
        if x != y {
            return *x as i32 - *y as i32;
        }
    }
    a.len() as i32 - b.len() as i32
}

/// `Character.toUpperCase(char)`: the simple (one to one) mapping of
/// UnicodeData. Rust only offers the full mapping, which equals the simple
/// one except for the characters it expands; of those, only the Greek
/// letters with ypogegrammeni have a simple uppercase (their titlecase
/// form), the others (`ß`, `ŉ`, ligatures, …) map to themselves.
fn upper(c: u16) -> u16 {
    match c {
        0x1F80..=0x1F87 | 0x1F90..=0x1F97 | 0x1FA0..=0x1FA7 => c + 8,
        0x1FB3 | 0x1FC3 | 0x1FF3 => c + 9,
        _ => simple_map(c, |ch| ch.to_uppercase().collect()),
    }
}

/// `Character.toLowerCase(char)`. The only full lowercase mapping that
/// expands is U+0130 (`İ` → `i̇`), whose simple lowercase is `i`.
fn lower(c: u16) -> u16 {
    match c {
        0x0130 => 0x0069,
        _ => simple_map(c, |ch| ch.to_lowercase().collect()),
    }
}

fn simple_map(c: u16, f: impl Fn(char) -> Vec<char>) -> u16 {
    let Some(ch) = char::from_u32(c as u32) else {
        return c;
    };
    match f(ch).as_slice() {
        [m] if (*m as u32) <= 0xFFFF => *m as u32 as u16,
        _ => c,
    }
}

/// `String.compareToIgnoreCase`.
pub fn compare_ignore_case(a: &[u16], b: &[u16]) -> i32 {
    for (&x, &y) in a.iter().zip(b) {
        if x != y {
            let (ux, uy) = (upper(x), upper(y));
            if ux != uy {
                let (lx, ly) = (lower(ux), lower(uy));
                if lx != ly {
                    return lx as i32 - ly as i32;
                }
            }
        }
    }
    a.len() as i32 - b.len() as i32
}

pub fn ordering(c: i32) -> Ordering {
    c.cmp(&0)
}

/// `String.equalsIgnoreCase`.
pub fn equals_ignore_case(a: &[u16], b: &[u16]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(&x, &y)| x == y || upper(x) == upper(y) || lower(upper(x)) == lower(upper(y)))
}

/// `String.indexOf(str, fromIndex)`.
pub fn index_of(hay: &[u16], needle: &[u16], from: i32) -> i32 {
    let from = from.max(0) as usize;
    if from > hay.len() {
        return if needle.is_empty() {
            hay.len() as i32
        } else {
            -1
        };
    }
    if needle.is_empty() {
        return from as i32;
    }
    if needle.len() > hay.len() {
        return -1;
    }
    (from..=hay.len() - needle.len())
        .find(|&i| &hay[i..i + needle.len()] == needle)
        .map_or(-1, |i| i as i32)
}

/// `String.lastIndexOf(str, fromIndex)`.
pub fn last_index_of(hay: &[u16], needle: &[u16], from: i32) -> i32 {
    let Some(right) = hay.len().checked_sub(needle.len()) else {
        return -1;
    };
    if from < 0 {
        return -1;
    }
    let from = (from as usize).min(right);
    (0..=from)
        .rev()
        .find(|&i| &hay[i..i + needle.len()] == needle)
        .map_or(-1, |i| i as i32)
}

#[cfg(test)]
pub fn replace(hay: &[u16], target: &[u16], with: &[u16]) -> Vec<u16> {
    let mut out = Vec::new();
    replace_into(&mut out, hay, target, with);
    out
}

/// The length of [`replace_into`]'s result (saturating), so that the
/// caller can check and reserve it first.
pub fn replaced_len(hay: &[u16], target: &[u16], with: &[u16]) -> usize {
    let matches = if target.is_empty() {
        hay.len() + 1
    } else {
        let mut count = 0;
        let mut i = 0;
        while i < hay.len() {
            if hay[i..].starts_with(target) {
                count += 1;
                i += target.len();
            } else {
                i += 1;
            }
        }
        count
    };
    // The matches don't overlap, so they fit in `hay`.
    (hay.len() - matches * target.len()).saturating_add(matches.saturating_mul(with.len()))
}

/// `String.replace(CharSequence, CharSequence)`: every occurrence,
/// literally, appended to `out`.
pub fn replace_into(out: &mut Vec<u16>, hay: &[u16], target: &[u16], with: &[u16]) {
    if target.is_empty() {
        for &c in hay {
            out.extend_from_slice(with);
            out.push(c);
        }
        out.extend_from_slice(with);
        return;
    }
    let mut i = 0;
    while i < hay.len() {
        if hay[i..].starts_with(target) {
            out.extend_from_slice(with);
            i += target.len();
        } else {
            out.push(hay[i]);
            i += 1;
        }
    }
}

/// `String.trim`: removes code units ≤ U+0020 at both ends.
pub fn trim(s: &[u16]) -> &[u16] {
    let start = s.iter().position(|&c| c > 0x20).unwrap_or(s.len());
    let end = s.iter().rposition(|&c| c > 0x20).map_or(start, |e| e + 1);
    &s[start..end]
}

/// Maps the well-formed runs of `s` with `f`, keeping unpaired surrogates.
fn map_text(s: &[u16], f: impl Fn(&str) -> String) -> Vec<u16> {
    let mut out = Vec::with_capacity(s.len());
    let mut run = String::new();
    for c in char::decode_utf16(s.iter().copied()) {
        match c {
            Ok(ch) => run.push(ch),
            Err(e) => {
                out.extend(f(&run).encode_utf16());
                run.clear();
                out.push(e.unpaired_surrogate());
            }
        }
    }
    out.extend(f(&run).encode_utf16());
    out
}

/// `String.toUpperCase()` with the root-locale (Unicode default) mapping.
pub fn to_upper(s: &[u16]) -> Vec<u16> {
    map_text(s, str::to_uppercase)
}

/// `String.toLowerCase()` with the root-locale (Unicode default) mapping.
pub fn to_lower(s: &[u16]) -> Vec<u16> {
    map_text(s, str::to_lowercase)
}

/// `String.getBytes("UTF-8")`: unpaired surrogates become `?`.
pub fn utf8_bytes(s: &[u16]) -> Vec<u8> {
    to_rust(s).into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(s: &str) -> Vec<u16> {
        to_utf16(s)
    }

    #[test]
    fn java_string_operations() {
        assert_eq!(compare_to(&u("a"), &u("b")), -1);
        assert_eq!(compare_to(&u("abc"), &u("ab")), 1);
        assert_eq!(compare_to(&u("B"), &u("a")), -31);
        assert_eq!(compare_ignore_case(&u("B"), &u("a")), 1);
        assert_eq!(compare_ignore_case(&u("ABC"), &u("abc")), 0);
        assert!(equals_ignore_case(&u("HeLLo"), &u("hello")));
        assert_eq!(index_of(&u("hello"), &u("l"), 0), 2);
        assert_eq!(index_of(&u("hello"), &u("l"), 3), 3);
        assert_eq!(index_of(&u("hello"), &u("z"), 0), -1);
        assert_eq!(index_of(&u("hello"), &u(""), 9), 5);
        assert_eq!(last_index_of(&u("hello"), &u("l"), 5), 3);
        assert_eq!(last_index_of(&u("hello"), &u("l"), 2), 2);
        assert_eq!(last_index_of(&u("hello"), &u("l"), -1), -1);
        assert_eq!(replace(&u("aXbXc"), &u("X"), &u("--")), u("a--b--c"));
        assert_eq!(replace(&u("abc"), &u(""), &u("-")), u("-a-b-c-"));
        for (hay, target, with) in [
            ("aXbXc", "X", "--"),
            ("abc", "", "-"),
            ("", "", "-"),
            ("aaaa", "aa", ""),
            ("aaa", "aa", "b"),
            ("abc", "z", "yy"),
        ] {
            let (hay, target, with) = (u(hay), u(target), u(with));
            assert_eq!(
                replaced_len(&hay, &target, &with),
                replace(&hay, &target, &with).len()
            );
        }
        assert_eq!(trim(&u(" \t x y \n")), u("x y").as_slice());
        assert_eq!(trim(&u("   ")), u("").as_slice());
        assert_eq!(to_upper(&u("straße")), u("STRASSE"));
        // Final sigma, as Java's `toLowerCase`.
        assert_eq!(to_lower(&u("ΟΔΟΣ")), u("\u{3bf}\u{3b4}\u{3bf}\u{3c2}"));
        assert_eq!(to_rust(&[0x61, 0xD800, 0x62]), "a?b");
        assert_eq!(to_upper(&[0x61, 0xD800]), vec![0x41, 0xD800]);
    }

    #[test]
    fn case_insensitive_comparison_uses_java_simple_mappings() {
        // Character.toLowerCase('İ') is 'i'.
        assert!(equals_ignore_case(&u("\u{130}"), &u("i")));
        assert!(equals_ignore_case(&u("\u{130}stanbul"), &u("ISTANBUL")));
        assert_eq!(compare_ignore_case(&u("\u{130}b"), &u("ia")), 1);
        // Characters whose full mapping expands but have no simple one.
        assert!(!equals_ignore_case(&u("\u{df}"), &u("ss")));
        assert_eq!(upper(0xDF), 0xDF);
        assert_eq!(upper(0xFB01), 0xFB01);
        // Ypogegrammeni letters map to their titlecase form.
        assert_eq!(upper(0x1FB3), 0x1FBC);
        assert_eq!(upper(0x1F80), 0x1F88);
        assert!(equals_ignore_case(&u("\u{1fb3}"), &u("\u{1fbc}")));
    }
}
