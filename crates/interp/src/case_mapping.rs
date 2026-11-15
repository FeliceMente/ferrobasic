//! Unicode 14 casing for the B4J recording profile (JDK 19).

#[path = "case_tables.rs"]
mod case_tables;

use case_tables::{CASE_IGNORABLE, CASED, LOWER, UPPER, UPPER_EXPANSIONS};

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

pub(super) fn is_cased(c: char) -> bool {
    has_property(c, CASED)
}

pub(super) fn is_case_ignorable(c: char) -> bool {
    has_property(c, CASE_IGNORABLE)
}

fn has_property(c: char, ranges: &[(u32, u32)]) -> bool {
    let c = u32::from(c);
    let i = ranges.partition_point(|&(_, end)| end < c);
    ranges.get(i).is_some_and(|&(start, _)| start <= c)
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
        for table in [CASED, CASE_IGNORABLE] {
            assert_sorted_disjoint(table.iter().copied());
        }
        assert!(
            UPPER_EXPANSIONS
                .windows(2)
                .all(|pair| pair[0].0 < pair[1].0)
        );
    }
}
