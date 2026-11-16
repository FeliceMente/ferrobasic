//! Unicode 14 casing for the B4J recording profile (JDK 19).

#[path = "case_tables.rs"]
mod case_tables;

use case_tables::{GENERAL_CATEGORIES, LOWER, UPPER, UPPER_EXPANSIONS};

/// A range of mapped code points, with a stride and an offset to the result.
/// The ranges are sorted and do not overlap. A stride of two skips the
/// intervening characters in blocks with alternating case.
type Range = (u32, u32, u32, i32);

pub(super) fn upper(c: u32) -> u32 {
    mapped(c, UPPER)
}

pub(super) fn lower(c: u32) -> u32 {
    mapped(c, LOWER)
}

fn mapped(c: u32, ranges: &[Range]) -> u32 {
    let i = ranges.partition_point(|&(_, end, _, _)| end < c);
    match ranges.get(i) {
        Some(&(start, _, stride, offset)) if c >= start && (c - start) % stride == 0 => {
            c.wrapping_add_signed(offset)
        }
        _ => c,
    }
}

pub(super) fn upper_expansion(c: char) -> Option<&'static str> {
    let i = UPPER_EXPANSIONS
        .binary_search_by_key(&c, |&(key, _)| key)
        .ok()?;
    Some(UPPER_EXPANSIONS[i].1)
}

/// A Unicode general category, as `Character.getType` reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Gc {
    Lu,
    Ll,
    Lt,
    Lm,
    Lo,
    Mn,
    Mc,
    Me,
    Nd,
    Nl,
    No,
    Pc,
    Pd,
    Ps,
    Pe,
    Pi,
    Pf,
    Po,
    Sm,
    Sc,
    Sk,
    So,
    Zs,
    Zl,
    Zp,
    Cc,
    Cf,
    Cs,
    Co,
    Cn,
}

/// The general category of a code point, or of a lone surrogate (`Cs`).
pub(super) fn general_category(c: u32) -> Gc {
    let i = GENERAL_CATEGORIES.partition_point(|&(_, end, _)| end < c);
    match GENERAL_CATEGORIES.get(i) {
        Some(&(start, _, category)) if start <= c => category,
        _ => Gc::Cn,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_sorted_disjoint(ranges: impl Iterator<Item = (u32, u32)>) {
        let mut last_end = None;
        for (start, end) in ranges {
            assert!(start <= end && end <= 0x10FFFF);
            assert!(last_end.is_none_or(|previous| previous < start));
            last_end = Some(end);
        }
    }

    #[test]
    fn generated_case_tables_support_binary_search() {
        for table in [UPPER, LOWER] {
            assert_sorted_disjoint(table.iter().map(|&(start, end, _, _)| (start, end)));
            for &(start, end, stride, _) in table {
                assert!((1..=2).contains(&stride));
                assert_eq!((end - start) % stride, 0);
            }
        }
        assert_sorted_disjoint(
            GENERAL_CATEGORIES
                .iter()
                .map(|&(start, end, _)| (start, end)),
        );
        assert!(
            UPPER_EXPANSIONS
                .windows(2)
                .all(|pair| pair[0].0 < pair[1].0)
        );
    }

    #[test]
    fn general_categories() {
        assert_eq!(general_category('A' as u32), Gc::Lu);
        assert_eq!(general_category(0x3A3), Gc::Lu);
        assert_eq!(general_category(0xAA), Gc::Lo);
        assert_eq!(general_category(0x4E00), Gc::Lo);
        assert_eq!(general_category(0x9FFF), Gc::Lo);
        assert_eq!(general_category(0xD800), Gc::Cs);
        assert_eq!(general_category(0x0378), Gc::Cn);
        assert_eq!(general_category(0x10FFFF), Gc::Cn);
        // U+1C8A is new in Unicode 16.
        assert_eq!(general_category(0x1C8A), Gc::Cn);
    }
}
