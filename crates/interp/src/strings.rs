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

/// `String.toUpperCase()` / `String.toLowerCase()` with the root-locale
/// (Unicode default) mapping, exactly as Rust's `str::to_uppercase` /
/// `str::to_lowercase` map each run between unpaired surrogates (which are
/// kept). The text is mapped one character at a time into a buffer that
/// grows fallibly: `None` if memory runs out, or if the result (which can be
/// longer, `ß` → `SS`) would pass `limit` code units.
pub fn change_case(s: &[u16], upper: bool, limit: usize) -> Option<Vec<u16>> {
    let mut out = Vec::new();
    out.try_reserve(s.len()).ok()?;
    let mut context = SigmaContext::default();
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
            Ok(c) if upper => c.to_uppercase().try_for_each(|c| push(&mut out, c))?,
            // `str::to_lowercase`'s one context rule: a capital sigma that
            // ends a word becomes the final form.
            Ok('Σ') => push(&mut out, context.lower_sigma(s, i))?,
            Ok(c) => c.to_lowercase().try_for_each(|c| push(&mut out, c))?,
        }
        i += width;
    }
    Some(out)
}

/// Decides between `σ` and `ς` as `str::to_lowercase` does: final when the
/// nearest character before that is not case-ignorable is cased, and the
/// nearest one after is not. Rust does not expose those two Unicode
/// properties, so they are read off `str::to_lowercase` itself (and cached).
#[derive(Default)]
struct SigmaContext {
    /// Whether a character is case-ignorable, and whether it is cased.
    known: std::collections::HashMap<char, (bool, bool)>,
}

impl SigmaContext {
    fn lower_sigma(&mut self, s: &[u16], at: usize) -> char {
        let before = chars_before(s, at);
        let after = char::decode_utf16(s[at + 1..].iter().copied()).map_while(Result::ok);
        if self.cased_beyond_ignorable(before) && !self.cased_beyond_ignorable(after) {
            'ς'
        } else {
            'σ'
        }
    }

    /// Whether the first character that is not case-ignorable is cased.
    fn cased_beyond_ignorable(&mut self, mut chars: impl Iterator<Item = char>) -> bool {
        chars
            .find_map(|c| {
                let (ignorable, cased) = self.properties(c);
                (!ignorable).then_some(cased)
            })
            .unwrap_or(false)
    }

    fn properties(&mut self, c: char) -> (bool, bool) {
        if let Some(&known) = self.known.get(&c) {
            return known;
        }
        let final_after = |text: String| text.to_lowercase().ends_with('ς');
        // `cΣ` ends in `ς` when `c` is cased and not case-ignorable; `AcΣ`
        // does when `c` is either (it is skipped, reaching `A`).
        let cased_not_ignorable = final_after(format!("{c}Σ"));
        let either = final_after(format!("A{c}Σ"));
        let properties = (either && !cased_not_ignorable, cased_not_ignorable);
        // The cache is only an optimisation: without room, skip it.
        if self.known.try_reserve(1).is_ok() {
            self.known.insert(c, properties);
        }
        properties
    }
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

    /// Case mapping one character at a time gives exactly what Rust's `str`
    /// functions give on whole runs, final sigma included.
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
