//! Java `String` operations on UTF-16 code units.

use std::cmp::Ordering;

use crate::case_mapping::{is_case_ignorable, is_cased, lower, upper, upper_expansion};

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

fn case_insensitive_difference(a: u32, b: u32) -> i32 {
    let (a, b) = (upper(a), upper(b));
    if a == b {
        return 0;
    }
    lower(a) as i32 - lower(b) as i32
}

/// Resolve a mismatched code unit to its code point. The high surrogate
/// may have matched on the previous iteration. A lone surrogate keeps
/// its code-unit value.
fn code_point_including(text: &[u16], index: &mut usize) -> u32 {
    let unit = text[*index];
    let (high, low) = match unit {
        0xD800..=0xDBFF => match text.get(*index + 1).copied() {
            Some(low @ 0xDC00..=0xDFFF) => {
                *index += 1;
                (unit, low)
            }
            _ => return u32::from(unit),
        },
        0xDC00..=0xDFFF if *index > 0 => match text[*index - 1] {
            high @ 0xD800..=0xDBFF => (high, unit),
            _ => return u32::from(unit),
        },
        _ => return u32::from(unit),
    };
    0x10000 + (u32::from(high - 0xD800) << 10) + u32::from(low - 0xDC00)
}

/// `String.compareToIgnoreCase`. Like the JDK's UTF-16 comparison, skip
/// matching code units and compare the containing code points on a mismatch.
pub fn compare_ignore_case(a: &[u16], b: &[u16]) -> i32 {
    let (mut i, mut j) = (0, 0);
    while let (Some(&x), Some(&y)) = (a.get(i), b.get(j)) {
        if x != y {
            let x = code_point_including(a, &mut i);
            let y = code_point_including(b, &mut j);
            let difference = case_insensitive_difference(x, y);
            if difference != 0 {
                return difference;
            }
        }
        i += 1;
        j += 1;
    }
    a.len() as i32 - b.len() as i32
}

pub fn ordering(c: i32) -> Ordering {
    c.cmp(&0)
}

/// `String.equalsIgnoreCase`.
pub fn equals_ignore_case(a: &[u16], b: &[u16]) -> bool {
    a.len() == b.len() && compare_ignore_case(a, b) == 0
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

/// `String.toUpperCase()` / `String.toLowerCase()` with the root-locale
/// (Unicode 14 default) mapping. Unpaired surrogates are kept.
/// The text is mapped one character at a time into a buffer that
/// grows fallibly: `None` if memory runs out, or if the result (which can be
/// longer, `ß` → `SS`) would pass `limit` code units.
pub fn change_case(s: &[u16], uppercase: bool, limit: usize) -> Option<Vec<u16>> {
    let mut out = Vec::new();
    out.try_reserve(s.len()).ok()?;
    let mut i = 0;
    for c in char::decode_utf16(s.iter().copied()) {
        let width = c.as_ref().map_or(1, |c| c.len_utf16());
        let mut units = [0u16; 2];
        let mut push = |out: &mut Vec<u16>, c: char| -> Option<()> {
            let c = c.encode_utf16(&mut units);
            if out.len() + c.len() > limit {
                return None;
            }
            out.try_reserve(c.len()).ok()?;
            out.extend_from_slice(c);
            Some(())
        };
        match c {
            Err(e) => {
                if out.len() >= limit {
                    return None;
                }
                out.try_reserve(1).ok()?;
                out.push(e.unpaired_surrogate());
            }
            Ok(c) if uppercase => match upper_expansion(c) {
                Some(expansion) => expansion.chars().try_for_each(|c| push(&mut out, c))?,
                None => push(&mut out, char::from_u32(upper(c as u32)).unwrap())?,
            },
            Ok('Σ') => push(&mut out, lower_sigma(s, i))?,
            Ok('\u{130}') => {
                push(&mut out, 'i')?;
                push(&mut out, '\u{307}')?;
            }
            Ok(c) => push(&mut out, char::from_u32(lower(c as u32)).unwrap())?,
        }
        i += width;
    }
    Some(out)
}

/// Use the final form when a cased character precedes sigma and none
/// follows it, skipping case-ignorable characters in both directions.
fn lower_sigma(s: &[u16], at: usize) -> char {
    let before = chars_before(s, at);
    let after = char::decode_utf16(s[at + 1..].iter().copied()).map_while(Result::ok);
    if cased_beyond_ignorable(before) && !cased_beyond_ignorable(after) {
        'ς'
    } else {
        'σ'
    }
}

fn cased_beyond_ignorable(mut chars: impl Iterator<Item = char>) -> bool {
    chars.find(|&c| !is_case_ignorable(c)).is_some_and(is_cased)
}

/// The characters of `s` before `at`, backwards, up to an unpaired
/// surrogate (where the run ends).
fn chars_before(s: &[u16], mut at: usize) -> impl Iterator<Item = char> + '_ {
    std::iter::from_fn(move || {
        let unit = *s.get(at.checked_sub(1)?)?;
        if (0xDC00..0xE000).contains(&unit) && at >= 2 && (0xD800..0xDC00).contains(&s[at - 2]) {
            at -= 2;
            char::decode_utf16([s[at], unit]).next()?.ok()
        } else {
            at -= 1;
            char::from_u32(unit as u32)
        }
    })
}

/// The previous implementation, kept as the reference for the tests.
#[cfg(test)]
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

#[cfg(test)]
pub fn to_upper(s: &[u16]) -> Vec<u16> {
    change_case(s, true, usize::MAX).unwrap()
}

#[cfg(test)]
pub fn to_lower(s: &[u16]) -> Vec<u16> {
    change_case(s, false, usize::MAX).unwrap()
}

/// `String.getBytes("UTF-8")`, byte by byte: unpaired surrogates become
/// `?`.
pub fn utf8_bytes(s: &[u16]) -> impl Iterator<Item = u8> {
    char::decode_utf16(s.iter().copied()).flat_map(|c| {
        let mut utf8 = [0; 4];
        let len = c.unwrap_or('?').encode_utf8(&mut utf8).len();
        utf8.into_iter().take(len)
    })
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
        let mut text = u("aé€😀");
        text.insert(1, 0xDC00);
        assert_eq!(utf8_bytes(&text).collect::<Vec<_>>(), "a?é€😀".as_bytes());
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
        // Expanding mappings count against the limit.
        assert_eq!(
            change_case(&u("\u{390}"), true, 3).map(|v| v.len()),
            Some(3)
        );
        assert_eq!(change_case(&u("\u{390}"), true, 2), None);
        assert_eq!(change_case(&u("ß"), true, 1), None);
        assert_eq!(change_case(&[0xDF, 0xD800], true, 2), None);
        assert_eq!(
            change_case(&[0xDF, 0xD800], true, 3),
            Some(vec![0x53, 0x53, 0xD800])
        );
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

    #[test]
    fn case_insensitive_comparison_preserves_surrogates() {
        // Matching high surrogates require looking back from different lows.
        assert!(equals_ignore_case(&[0xD801, 0xDC00], &[0xD801, 0xDC28]));
        assert!(compare_ignore_case(&[0xD801, 0xDC00], &[0xD801]) > 0);
        assert!(compare_ignore_case(&[0xD801, 0xDC00], &[0xD800]) > 0);
        assert!(equals_ignore_case(&[0x61, 0xD801], &[0x41, 0xD801]));
        assert!(compare_ignore_case(&[0x41, 0xD801], &[0x61, 0xD802]) < 0);
    }

    #[test]
    fn case_mapping_uses_the_recording_jdks_unicode_version() {
        for (uppercase, lowercase) in [
            ("\u{10400}", "\u{10428}"), // Deseret
            ("\u{104B0}", "\u{104D8}"), // Osage
            ("\u{10570}", "\u{10597}"), // Vithkuqi
            ("\u{10C80}", "\u{10CC0}"), // Old Hungarian
            ("\u{118A0}", "\u{118C0}"), // Warang Citi
            ("\u{16E40}", "\u{16E60}"), // Medefaidrin
            ("\u{1E900}", "\u{1E922}"), // Adlam
        ] {
            let (uppercase, lowercase) = (u(uppercase), u(lowercase));
            assert!(equals_ignore_case(&uppercase, &lowercase));
            assert_eq!(to_upper(&lowercase), uppercase);
            assert_eq!(to_lower(&uppercase), lowercase);
        }
        for (uppercase, lowercase) in [
            ("\u{1C89}", "\u{1C8A}"),
            ("\u{A7CB}", "\u{0264}"),
            ("\u{10D50}", "\u{10D70}"), // Garay
        ] {
            let (uppercase, lowercase) = (u(uppercase), u(lowercase));
            assert!(!equals_ignore_case(&uppercase, &lowercase));
            assert_eq!(to_upper(&lowercase), lowercase);
            assert_eq!(to_lower(&uppercase), uppercase);
        }
    }

    #[test]
    fn final_sigma_uses_the_recording_jdks_unicode_properties() {
        // U+1C8A (Unicode 16) does not count as cased in Unicode 14.
        assert_eq!(to_lower(&u("AΣ\u{1C8A}")), u("aς\u{1C8A}"));
        assert_eq!(to_lower(&u("\u{1C8A}Σ")), u("\u{1C8A}σ"));
        // Skip a combining mark and continue to the cased letter.
        assert_eq!(to_lower(&u("AΣ\u{301}B")), u("aσ\u{301}b"));
        assert_eq!(to_lower(&u("A\u{301}Σ")), u("a\u{301}ς"));
    }

    /// For characters in both Unicode versions, exercise whole-run
    /// mapping and the final sigma rule.
    #[test]
    fn change_case_matches_whole_run_mapping() {
        let pool: Vec<Vec<u16>> = [
            "a",
            "Z",
            " ",
            ".",
            "'",
            "1",
            "\u{3a3}",
            "\u{3c3}",
            "\u{391}",
            "\u{390}",
            "\u{301}",
            "\u{345}",
            "\u{2b0}",
            "\u{ad}",
            "\u{df}",
            "\u{130}",
            "\u{1f600}",
        ]
        .iter()
        .map(|t| u(t))
        .chain([vec![0xD800], vec![0xDC00]])
        .collect();
        let mut seed: u64 = 42;
        for _ in 0..20_000 {
            let mut text = Vec::new();
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            for _ in 0..(seed >> 60) {
                seed = seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                text.extend(&pool[(seed >> 33) as usize % pool.len()]);
            }
            assert_eq!(
                to_lower(&text),
                map_text(&text, str::to_lowercase),
                "{text:x?}"
            );
            assert_eq!(
                to_upper(&text),
                map_text(&text, str::to_uppercase),
                "{text:x?}"
            );
        }
    }
}
