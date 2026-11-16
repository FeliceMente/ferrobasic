//! Java's final-sigma rule for `toLowerCase` (JDK 19), with the word
//! boundaries of its legacy word `BreakIterator`.
//!
//! `ConditionalSpecialCasing.isFinalCased` lowercases `Σ` to `ς` when,
//! within the closest word boundaries around it, a cased character
//! precedes it and none follows it. Its word boundaries come from
//! `BreakIterator.getWordInstance`, whose default rules are the
//! `WordBreakRules` of `sun.text.resources.BreakIteratorRules`, not
//! Unicode's word segmentation. Recording run 23
//! (`lowercase_final_sigma_context`) confirms B4J 10.70 follows them.
//!
//! The rules are compiled here into a small automaton that is run the way
//! `RuleBasedBreakIterator.handleNext` runs its state table: each segment
//! advances at least one character, takes the longest match of any rule,
//! and skips format characters (`Cf`) without changing state.

use std::sync::OnceLock;

use crate::case_mapping::{Gc, general_category};

/// Whether `Σ` at `at` in `s` takes the final form `ς`, given the word
/// boundaries of `s` (from [`word_boundaries`]).
pub(crate) fn is_final_cased(s: &[u16], at: usize, boundaries: &[bool]) -> bool {
    // Java asks `isBoundary(i)`, which is `following(i - 1) == i`. After a
    // surrogate pair, `i - 1` falls inside the pair: `following` backs up
    // from the lone low surrogate, and the segment it then reads ends at
    // `i`. So a position right after a pair always reads as a boundary,
    // unless the pair starts the text, where the backup reaches the start
    // and the real segmentation applies.
    let boundary = |i: usize| boundaries[i] || (i > 2 && code_point_before(s, i).1 == 2);
    let mut i = at;
    while !boundary(i) {
        let (c, width) = code_point_before(s, i);
        if is_cased(c) {
            let mut j = at + 1;
            while j < s.len() && !boundary(j) {
                let (c, width) = code_point_at(s, j);
                if is_cased(c) {
                    return false;
                }
                j += width;
            }
            return true;
        }
        i -= width;
    }
    false
}

/// `ConditionalSpecialCasing.isCased`: the letter categories `Lu`, `Ll`
/// and `Lt`, plus JDK 19's fixed list of `Other_Lowercase` and
/// `Other_Uppercase` ranges. `ª` (`Lo`) is not cased.
fn is_cased(c: u32) -> bool {
    matches!(general_category(c), Gc::Lu | Gc::Ll | Gc::Lt)
        || matches!(
            c,
            0x02B0..=0x02B8
                | 0x02C0..=0x02C1
                | 0x02E0..=0x02E4
                | 0x0345
                | 0x037A
                | 0x1D2C..=0x1D61
                | 0x2160..=0x217F
                | 0x24B6..=0x24E9
        )
}

/// For each UTF-16 offset of `s` (and its end), whether a word boundary
/// falls there.
pub(crate) fn word_boundaries(s: &[u16]) -> Vec<bool> {
    let mut points = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let (c, width) = code_point_at(s, i);
        points.push((c, i));
        i += width;
    }
    let mut boundaries = vec![false; s.len() + 1];
    boundaries[0] = true;
    boundaries[s.len()] = true;
    let nfa = rules();
    let mut start = 0;
    while start < points.len() {
        let end = nfa.segment(&points, start);
        boundaries[points.get(end).map_or(s.len(), |&(_, at)| at)] = true;
        start = end;
    }
    boundaries
}

/// `String.codePointAt`: a lone surrogate is its own code point.
fn code_point_at(s: &[u16], i: usize) -> (u32, usize) {
    let unit = s[i];
    match s.get(i + 1) {
        Some(&low) if (0xD800..0xDC00).contains(&unit) && (0xDC00..0xE000).contains(&low) => (
            0x10000 + ((u32::from(unit) - 0xD800) << 10) + (u32::from(low) - 0xDC00),
            2,
        ),
        _ => (u32::from(unit), 1),
    }
}

/// `String.codePointBefore`.
fn code_point_before(s: &[u16], i: usize) -> (u32, usize) {
    if i >= 2 && (0xDC00..0xE000).contains(&s[i - 1]) && (0xD800..0xDC00).contains(&s[i - 2]) {
        code_point_at(s, i - 2)
    } else {
        (u32::from(s[i - 1]), 1)
    }
}

// The character sets of `WordBreakRules`, by name.

/// `<ignore>`: format characters, which never change the state, except
/// the soft hyphen, which `<mid-word>` names, and supplementary ones,
/// which JDK 19 does not treat as ignorable.
fn ignore(c: u32) -> bool {
    c < 0x10000 && c != 0x00AD && general_category(c) == Gc::Cf
}

fn enclosing(c: u32) -> bool {
    matches!(general_category(c), Gc::Mn | Gc::Me)
}

fn danda(c: u32) -> bool {
    matches!(c, 0x0964 | 0x0965)
}

fn kanji(c: u32) -> bool {
    matches!(c, 0x3005 | 0x4E00..=0x9FA5 | 0xF900..=0xFA2D)
}

fn kata(c: u32) -> bool {
    matches!(c, 0x30A1..=0x30FA | 0x30FD | 0x30FE)
}

fn hira(c: u32) -> bool {
    matches!(c, 0x3041..=0x3094 | 0x309D | 0x309E)
}

fn cjk_diacrit(c: u32) -> bool {
    matches!(c, 0x3099..=0x309C | 0x30FB | 0x30FC)
}

fn letter_base(c: u32) -> bool {
    matches!(
        general_category(c),
        Gc::Lu | Gc::Ll | Gc::Lt | Gc::Lm | Gc::Lo | Gc::Mc
    ) && !(kanji(c) || kata(c) || hira(c) || cjk_diacrit(c))
}

fn digit_base(c: u32) -> bool {
    matches!(general_category(c), Gc::Nd | Gc::Nl | Gc::No)
}

fn mid_word(c: u32) -> bool {
    matches!(general_category(c), Gc::Pd | Gc::Pc)
        || matches!(c, 0x00AD | 0x2027 | 0x22 | 0x27 | 0x2E)
}

fn mid_num(c: u32) -> bool {
    matches!(c, 0x22 | 0x27 | 0x2C | 0x066B | 0x2E)
}

fn pre_num(c: u32) -> bool {
    (general_category(c) == Gc::Sc || matches!(c, 0x23 | 0x2E)) && c != 0xA2
}

fn post_num(c: u32) -> bool {
    matches!(c, 0x25 | 0x26 | 0xA2 | 0x066A | 0x2030 | 0x2031)
}

fn line_separator(c: u32) -> bool {
    matches!(c, 0x0A | 0x0C | 0x2028 | 0x2029)
}

fn whitespace_base(c: u32) -> bool {
    general_category(c) == Gc::Zs || c == 0x09
}

fn carriage_return(c: u32) -> bool {
    c == 0x0D
}

fn kata_or_diacrit(c: u32) -> bool {
    kata(c) || cjk_diacrit(c)
}

fn hira_or_diacrit(c: u32) -> bool {
    hira(c) || cjk_diacrit(c)
}

fn base(c: u32) -> bool {
    !enclosing(c) && !matches!(general_category(c), Gc::Cc | Gc::Cf | Gc::Zl | Gc::Zp)
}

fn any(_: u32) -> bool {
    true
}

type Set = fn(u32) -> bool;

/// A rule, as a regular expression over character sets.
enum Re {
    Set(Set),
    Seq(Vec<Re>),
    Alt(Vec<Re>),
    Star(Box<Re>),
    Opt(Box<Re>),
}

fn star(re: Re) -> Re {
    Re::Star(Box::new(re))
}

fn opt(re: Re) -> Re {
    Re::Opt(Box::new(re))
}

/// `WordBreakRules`, in the order JDK 19 lists them.
fn word_rules() -> Re {
    use Re::{Alt, Seq, Set};
    let letter = || Seq(vec![Set(letter_base), star(Set(enclosing))]);
    let digit = || Seq(vec![Set(digit_base), star(Set(enclosing))]);
    // <word>=((<let><let>*(<mid-word><let><let>*)*){<danda>});
    let word = || {
        Seq(vec![
            letter(),
            star(letter()),
            star(Seq(vec![Set(mid_word), letter(), star(letter())])),
            opt(Set(danda)),
        ])
    };
    // <number>=(<dgt><dgt>*(<mid-num><dgt><dgt>*)*);
    let number = || {
        Seq(vec![
            digit(),
            star(digit()),
            star(Seq(vec![Set(mid_num), digit(), star(digit())])),
        ])
    };
    // (<number><word>)*{<number>{<post-num>}}
    let rest = || {
        Seq(vec![
            star(Seq(vec![number(), word()])),
            opt(Seq(vec![number(), opt(Set(post_num))])),
        ])
    };
    Alt(vec![
        // .;
        Set(any),
        // {<word>}(<number><word>)*{<number>{<post-num>}};
        Seq(vec![opt(word()), rest()]),
        // <pre-num>(<number><word>)*{<number>{<post-num>}};
        Seq(vec![Set(pre_num), rest()]),
        // <ws>*{\r}{<ls>};
        Seq(vec![
            star(Seq(vec![Set(whitespace_base), star(Set(enclosing))])),
            opt(Set(carriage_return)),
            opt(Set(line_separator)),
        ]),
        // [<kata><cjk-diacrit>]*;
        star(Set(kata_or_diacrit)),
        // [<hira><cjk-diacrit>]*;
        star(Set(hira_or_diacrit)),
        // <kanji>*;
        star(Set(kanji)),
        // <base><enclosing><enclosing>*;
        Seq(vec![Set(base), Set(enclosing), star(Set(enclosing))]),
    ])
}

enum State {
    Char(Set, usize),
    Split(usize, usize),
    Match,
}

/// A Thompson automaton for the union of the rules.
struct Nfa {
    states: Vec<State>,
    start: usize,
}

const MATCH: usize = 0;

fn rules() -> &'static Nfa {
    static NFA: OnceLock<Nfa> = OnceLock::new();
    NFA.get_or_init(|| {
        let mut states = vec![State::Match];
        let start = compile(&word_rules(), MATCH, &mut states);
        Nfa { states, start }
    })
}

/// Compile `re` so that it continues to `next`; returns its entry state.
fn compile(re: &Re, next: usize, states: &mut Vec<State>) -> usize {
    match re {
        Re::Set(set) => {
            states.push(State::Char(*set, next));
            states.len() - 1
        }
        Re::Seq(parts) => parts
            .iter()
            .rev()
            .fold(next, |next, part| compile(part, next, states)),
        Re::Alt(choices) => {
            let mut entries = choices.iter().map(|choice| compile(choice, next, states));
            let first = entries.next().unwrap_or(next);
            let rest: Vec<usize> = entries.collect();
            rest.into_iter().fold(first, |entry, other| {
                states.push(State::Split(entry, other));
                states.len() - 1
            })
        }
        Re::Star(body) => {
            states.push(State::Split(MATCH, next));
            let split = states.len() - 1;
            let entry = compile(body, split, states);
            states[split] = State::Split(entry, next);
            split
        }
        Re::Opt(body) => {
            let entry = compile(body, next, states);
            states.push(State::Split(entry, next));
            states.len() - 1
        }
    }
}

impl Nfa {
    /// Add `state` and the states reachable from it without input.
    fn close(&self, state: usize, set: &mut Vec<usize>, seen: &mut [bool]) {
        if std::mem::replace(&mut seen[state], true) {
            return;
        }
        match self.states[state] {
            State::Split(a, b) => {
                self.close(a, set, seen);
                self.close(b, set, seen);
            }
            _ => set.push(state),
        }
    }

    /// The end, as an index into `points`, of the segment that starts at
    /// `start`: `handleNext`, without lookahead (the word rules have none).
    fn segment(&self, points: &[(u32, usize)], start: usize) -> usize {
        let mut seen = vec![false; self.states.len()];
        let mut set = Vec::new();
        self.close(self.start, &mut set, &mut seen);
        let mut end = start + 1;
        for (i, &(c, _)) in points.iter().enumerate().skip(start) {
            if !ignore(c) {
                seen.fill(false);
                let mut next = Vec::new();
                for &state in &set {
                    if let State::Char(set, to) = self.states[state]
                        && set(c)
                    {
                        self.close(to, &mut next, &mut seen);
                    }
                }
                if next.is_empty() {
                    break;
                }
                set = next;
            }
            if set.contains(&MATCH) {
                end = end.max(i + 1);
            }
        }
        end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    fn breaks(s: &str) -> Vec<usize> {
        let b = word_boundaries(&u(s));
        (0..b.len()).filter(|&i| b[i]).collect()
    }

    #[test]
    fn word_boundaries_follow_the_legacy_rules() {
        assert_eq!(breaks(""), [0]);
        assert_eq!(breaks("ab cd"), [0, 2, 3, 5]);
        // Letters and digits stay together; mid-word punctuation joins
        // letters only.
        assert_eq!(breaks("A1Σ"), [0, 3]);
        assert_eq!(breaks("A_Σ"), [0, 3]);
        assert_eq!(breaks("A:Σ"), [0, 1, 2, 3]);
        assert_eq!(breaks("1.5%"), [0, 4]);
        assert_eq!(breaks("A1.Σ"), [0, 2, 3, 4]);
        assert_eq!(breaks("$12"), [0, 3]);
        // A format character joins whatever it follows, except the soft
        // hyphen and supplementary ones.
        assert_eq!(breaks("a\u{200B} b"), [0, 2, 3, 4]);
        assert_eq!(breaks("Σ\u{AD}"), [0, 1, 2]);
        assert_eq!(breaks("\u{E0001}#"), [0, 2, 3]);
        assert_eq!(breaks("A\u{10400}Σ."), [0, 4, 5]);
    }

    fn lower_sigma(text: &str) -> char {
        let s = u(text);
        let at = s.iter().position(|&c| c == 0x3A3).unwrap();
        if is_final_cased(&s, at, &word_boundaries(&s)) {
            'ς'
        } else {
            'σ'
        }
    }

    /// Cases where JDK 19 differs from Unicode's final-sigma context,
    /// checked against a JDK.
    #[test]
    fn final_sigma_follows_the_jdk() {
        // Word boundaries limit the context.
        assert_eq!(lower_sigma("A.Σ"), 'ς');
        assert_eq!(lower_sigma("A'Σ"), 'ς');
        assert_eq!(lower_sigma("A-Σ"), 'ς');
        assert_eq!(lower_sigma("1Σ"), 'σ');
        assert_eq!(lower_sigma("A1.Σ"), 'σ');
        assert_eq!(lower_sigma("AΣ B"), 'ς');
        // Java's own test for cased characters.
        assert_eq!(lower_sigma("ʰΣ"), 'ς');
        assert_eq!(lower_sigma("ⅰΣ"), 'ς');
        // A cased symbol is a word of its own.
        assert_eq!(lower_sigma("AΣⓐ"), 'ς');
        // After a surrogate pair, a position always reads as a boundary,
        // unless the pair starts the text.
        assert_eq!(lower_sigma("\u{10400}Σ"), 'ς');
        assert_eq!(lower_sigma("A\u{10400}Σ"), 'σ');
        assert_eq!(lower_sigma("AΣ\u{10400}"), 'σ');
        assert_eq!(lower_sigma("\u{10400}Σ\u{10428}"), 'σ');
    }

    #[test]
    fn final_sigma_matches_the_recording() {
        // lowercase_final_sigma_context, recording run 23.
        for (text, at, last) in [
            ("A1Σ", 2, true),
            ("A_Σ", 2, true),
            ("AΣ_B", 1, false),
            ("A:Σ", 2, false),
            ("ªΣ", 1, false),
        ] {
            let s = u(text);
            assert_eq!(is_final_cased(&s, at, &word_boundaries(&s)), last, "{text}");
        }
    }
}
