//! Number formatting and parsing with Java semantics: `Double.toString`
//! (JDK 19 and later), `BA.NumberToString`, `NumberFormat` (Java
//! `DecimalFormat` in the US locale), `Double.parseDouble` and
//! `java.util.Random`.

/// A positive, finite, non-zero number as decimal digits:
/// `d0.d1d2… × 10^exp`. `digits` has no leading or trailing zero.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Decimal {
    digits: Vec<u8>,
    exp: i32,
}

/// Parses Rust's `{:e}` output (`1.2345e4`, `5e-324`).
fn parse_sci(s: &str) -> Decimal {
    let (mantissa, exp) = s.split_once('e').unwrap_or((s, "0"));
    let mut digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).collect();
    while digits.len() > 1 && digits.last() == Some(&b'0') {
        digits.pop();
    }
    Decimal {
        digits,
        exp: exp.parse().unwrap_or(0),
    }
}

/// The digits Java's `Double.toString` / `Float.toString` select (JDK 19
/// javadoc): among the decimals of the shortest length that round to the
/// value, the closest one, and of two equally close, the one with an even
/// last digit; a single digit is lengthened to the closest decimal of length
/// 2, e.g. `4.9E-324` rather than `5E-324`.
///
/// Rust's `{:e}` gives the shortest length, but not always the closest
/// decimal of that length; `{:.N$e}` gives the closest, rounding the exact
/// value half-even, which is Java's tie rule.
fn java_digits(
    shortest: &str,
    closest: impl Fn(usize) -> String,
    round_trips: impl Fn(&str) -> bool,
) -> Decimal {
    let d1 = parse_sci(shortest);
    let len = d1.digits.len().max(2);
    let candidate = closest(len - 1);
    if round_trips(&candidate) {
        let d = parse_sci(&candidate);
        if d.digits.len() == len {
            return d;
        }
    }
    d1
}

fn digits_f64(x: f64) -> Decimal {
    java_digits(
        &format!("{x:e}"),
        |p| format!("{x:.p$e}"),
        |s| s.parse::<f64>() == Ok(x),
    )
}

fn digits_f32(x: f32) -> Decimal {
    java_digits(
        &format!("{x:e}"),
        |p| format!("{x:.p$e}"),
        |s| s.parse::<f32>() == Ok(x),
    )
}

/// Java's layout: plain for 1e-3 ≤ |x| < 1e7, otherwise `d.dddE±n`, with at
/// least one digit after the point.
fn java_layout(negative: bool, d: &Decimal) -> String {
    let mut s = String::new();
    if negative {
        s.push('-');
    }
    let digits = |r: std::ops::Range<usize>| -> String {
        r.map(|i| *d.digits.get(i).unwrap_or(&b'0') as char)
            .collect()
    };
    let n = d.digits.len();
    if (-3..7).contains(&d.exp) {
        if d.exp >= 0 {
            let int_len = d.exp as usize + 1;
            s.push_str(&digits(0..int_len));
            s.push('.');
            if n > int_len {
                s.push_str(&digits(int_len..n));
            } else {
                s.push('0');
            }
        } else {
            s.push_str("0.");
            for _ in 0..(-d.exp - 1) {
                s.push('0');
            }
            s.push_str(&digits(0..n));
        }
    } else {
        s.push(d.digits[0] as char);
        s.push('.');
        if n > 1 {
            s.push_str(&digits(1..n));
        } else {
            s.push('0');
        }
        s.push('E');
        s.push_str(&d.exp.to_string());
    }
    s
}

/// Java `Double.toString` (JDK 19 and later).
pub fn java_double_to_string(x: f64) -> String {
    if x.is_nan() {
        "NaN".into()
    } else if x.is_infinite() {
        if x > 0.0 { "Infinity" } else { "-Infinity" }.into()
    } else if x == 0.0 {
        if x.is_sign_negative() { "-0.0" } else { "0.0" }.into()
    } else {
        java_layout(x < 0.0, &digits_f64(x.abs()))
    }
}

/// Java `Float.toString` (JDK 19 and later).
pub fn java_float_to_string(x: f32) -> String {
    if x.is_nan() {
        "NaN".into()
    } else if x.is_infinite() {
        if x > 0.0 { "Infinity" } else { "-Infinity" }.into()
    } else if x == 0.0 {
        if x.is_sign_negative() { "-0.0" } else { "0.0" }.into()
    } else {
        java_layout(x < 0.0, &digits_f32(x.abs()))
    }
}

/// `BA.NumberToString` for a Double: `Double.toString`, then a trailing
/// `.0` removed.
pub fn number_to_string(x: f64) -> String {
    let mut s = java_double_to_string(x);
    if s.ends_with(".0") {
        s.truncate(s.len() - 2);
    }
    s
}

/// The digits `DecimalFormat` formats a positive double from. Its
/// `DigitList` takes them from the JDK's older conversion
/// (`FloatingDecimal`), not from `Double.toString`. Measured against JDK 21,
/// the two agree except on:
///
/// - integers from 2^53 to 2^63: the older one gives the exact integer, with
///   its last 0, 1 or 2 digits rounded depending on the binary exponent
///   (`DROPPED_DIGITS`);
/// - other powers of two: it takes the shortest decimal within a quarter of
///   the gap above on both sides, where `Double.toString` allows half the
///   gap above (`power_of_two_digits`);
/// - subnormals, and a few other values (3 in 100,000 random doubles, all
///   above 2^63) where it gives 16 or 17 digits although fewer round-trip;
///   these are not reproduced (`docs/deviations.md`, D3).
fn format_digits(a: f64) -> Decimal {
    /// Low digits dropped, for binary exponents 53 to 62.
    const DROPPED_DIGITS: [u32; 10] = [0, 0, 0, 0, 0, 1, 1, 1, 2, 2];
    let bits = a.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32 - 1023;
    if !(53..63).contains(&exponent) {
        if bits & ((1 << 52) - 1) == 0 && exponent > -1022 {
            return power_of_two_digits(a, exponent);
        }
        return digits_f64(a);
    }
    // Integral: every double from 2^53 up is.
    let unit = 10u64.pow(DROPPED_DIGITS[exponent as usize - 53]);
    let n = a as u64;
    let rounded = (n / unit + u64::from(n % unit * 2 >= unit)) * unit;
    let text = rounded.to_string();
    let digits = text.trim_end_matches('0').bytes().collect();
    Decimal {
        digits,
        exp: text.len() as i32 - 1,
    }
}

/// The shortest decimal within 2^(e-54) of `a` = 2^e, and of those the
/// closest: `{:.N$e}` gives the closest of each length.
fn power_of_two_digits(a: f64, e: i32) -> Decimal {
    for precision in 0..17 {
        let candidate = parse_sci(&format!("{a:.precision$e}"));
        // candidate = m × 10^k
        let m = candidate
            .digits
            .iter()
            .fold(0u64, |m, d| m * 10 + u64::from(d - b'0'));
        let k = candidate.exp - (candidate.digits.len() as i32 - 1);
        // |m·10^k − 2^e| ≤ 2^(e−54), times 2^54:
        // 2^e·(2^54 − 1) ≤ m·10^k·2^54 ≤ 2^e·(2^54 + 1), in integers.
        let mut x = Big::from(m);
        let mut lo = Big::from((1 << 54) - 1);
        let mut hi = Big::from((1 << 54) + 1);
        x.shl(54 + (-e).max(0) as u32);
        lo.shl(e.max(0) as u32);
        hi.shl(e.max(0) as u32);
        for _ in 0..k.max(0) {
            x.mul_small(10);
        }
        for _ in 0..(-k).max(0) {
            lo.mul_small(10);
            hi.mul_small(10);
        }
        if lo.cmp(&x).is_le() && x.cmp(&hi).is_le() {
            return candidate;
        }
    }
    digits_f64(a)
}

/// A non-negative integer, as little-endian 32-bit limbs.
struct Big(Vec<u32>);

impl Big {
    fn from(n: u64) -> Big {
        Big(vec![n as u32, (n >> 32) as u32])
    }

    fn mul_small(&mut self, factor: u32) {
        let mut carry = 0u64;
        for limb in &mut self.0 {
            let v = u64::from(*limb) * u64::from(factor) + carry;
            *limb = v as u32;
            carry = v >> 32;
        }
        if carry > 0 {
            self.0.push(carry as u32);
        }
    }

    fn shl(&mut self, bits: u32) {
        let mut carry = 0u32;
        let shift = bits % 32;
        if shift > 0 {
            for limb in &mut self.0 {
                let v = *limb;
                *limb = (v << shift) | carry;
                carry = v >> (32 - shift);
            }
            self.0.push(carry);
        }
        self.0
            .splice(0..0, std::iter::repeat_n(0, (bits / 32) as usize));
    }

    fn cmp(&self, other: &Big) -> std::cmp::Ordering {
        let len = self.0.len().max(other.0.len());
        let limb = |b: &Big, i: usize| b.0.get(i).copied().unwrap_or(0);
        (0..len)
            .rev()
            .map(|i| limb(self, i).cmp(&limb(other, i)))
            .find(|o| o.is_ne())
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

/// Java `DecimalFormat` limits for doubles.
const DOUBLE_INTEGER_DIGITS: i32 = 309;
const DOUBLE_FRACTION_DIGITS: i32 = 340;

/// `NumberFormat.getInstance(Locale.US)` with the given settings, formatting
/// a double: HALF_EVEN rounding on the exact binary value, `,` grouping by
/// three, no integer digit printed for |x| < 1 when `min_int` is 0
/// (`.5`), `-0` for negative values that round to zero, `NaN`, `∞`.
///
/// `max_frac` and `min_frac` follow `setMaximumFractionDigits` then
/// `setMinimumFractionDigits`: the minimum wins when they conflict.
pub fn number_format(x: f64, min_int: i32, max_frac: i32, min_frac: i32, grouping: bool) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    let negative = x < 0.0 || (x == 0.0 && x.is_sign_negative());
    let sign = if negative { "-" } else { "" };
    if x.is_infinite() {
        return format!("{sign}\u{221e}");
    }
    let min_frac = min_frac.clamp(0, DOUBLE_FRACTION_DIGITS) as usize;
    let max_frac = (max_frac.clamp(0, DOUBLE_FRACTION_DIGITS) as usize).max(min_frac);
    let min_int = min_int.clamp(0, DOUBLE_INTEGER_DIGITS) as usize;
    let a = x.abs();

    // Integer and fraction digits of the digits Java's `DigitList` starts
    // from (see `format_digits`), rounded when they do not fit.
    let (mut int, mut frac) = (String::new(), String::new());
    if a != 0.0 {
        let d = format_digits(a);
        let n = d.digits.len() as i64;
        let point = d.exp as i64 + 1;
        for i in 0..point.max(0) {
            int.push(*d.digits.get(i as usize).unwrap_or(&b'0') as char);
        }
        for _ in point..0 {
            frac.push('0');
        }
        for i in point.max(0)..n {
            frac.push(d.digits[i as usize] as char);
        }
        if frac.len() > max_frac {
            let rounded = format!("{a:.max_frac$}");
            let (i, f) = rounded.split_once('.').unwrap_or((&rounded, ""));
            int = i.to_string();
            frac = f.to_string();
        }
    }
    let int = int.trim_start_matches('0');
    while frac.len() > min_frac && frac.ends_with('0') {
        frac.pop();
    }
    while frac.len() < min_frac {
        frac.push('0');
    }
    let mut int = format!("{}{int}", "0".repeat(min_int.saturating_sub(int.len())));
    if int.is_empty() && frac.is_empty() {
        int.push('0');
    }
    let mut out = String::from(sign);
    let len = int.len();
    for (i, c) in int.chars().enumerate() {
        if grouping && i > 0 && (len - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if !frac.is_empty() {
        out.push('.');
        out.push_str(&frac);
    }
    out
}

/// Java `Double.parseDouble`: surrounding characters ≤ U+0020 are ignored;
/// accepts an optional sign, `NaN`, `Infinity`, decimal literals with an
/// optional exponent, hexadecimal literals (`0x1.8p1`) and a trailing
/// `f`/`F`/`d`/`D`.
pub fn parse_java_double(text: &str) -> Option<f64> {
    let s = text.trim_matches(|c: char| c <= ' ');
    let (negative, body) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let value = match body {
        "NaN" => f64::NAN,
        "Infinity" => f64::INFINITY,
        _ => {
            let body = body.strip_suffix(['f', 'F', 'd', 'D']).unwrap_or(body);
            if let Some(hex) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
                parse_hex(hex)?
            } else {
                parse_decimal(body)?
            }
        }
    };
    Some(if negative { -value } else { value })
}

fn parse_decimal(s: &str) -> Option<f64> {
    let b = s.as_bytes();
    let mut i = 0;
    let mut digits = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
        digits += 1;
    }
    if i < b.len() && b[i] == b'.' {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
            digits += 1;
        }
    }
    if digits == 0 {
        return None;
    }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        i += 1;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            i += 1;
        }
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        if i == start {
            return None;
        }
    }
    if i != b.len() {
        return None;
    }
    s.parse().ok()
}

/// `HexDigits [. HexDigits] (p|P) [+|-] Digits`, correctly rounded
/// (half to even) to a double, subnormals included.
fn parse_hex(s: &str) -> Option<f64> {
    let (mantissa, exp) = s.split_once(['p', 'P'])?;
    // Exactly one optional sign, then at least one decimal digit. The value
    // saturates: anything beyond ±10^9 is far outside the double range.
    let (negative, digits) = match exp.as_bytes().first() {
        Some(b'-') => (true, &exp[1..]),
        Some(b'+') => (false, &exp[1..]),
        _ => (false, exp),
    };
    if digits.is_empty() || !digits.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let mut exp: i64 = 0;
    for c in digits.bytes() {
        exp = (exp * 10 + (c - b'0') as i64).min(1_000_000_000);
    }
    if negative {
        exp = -exp;
    }
    let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    if int.is_empty() && frac.is_empty() {
        return None;
    }
    // The first 60+ significant bits, exactly, and whether any non-zero
    // digit was dropped (`sticky`); the value is `value × 2^shift`.
    let mut value: u64 = 0;
    let mut shift: i64 = 0;
    let mut sticky = false;
    for (k, c) in int.chars().chain(frac.chars()).enumerate() {
        let d = c.to_digit(16)? as u64;
        if value >> 60 == 0 {
            value = value << 4 | d;
            if k >= int.len() {
                shift -= 4;
            }
        } else {
            sticky |= d != 0;
            if k < int.len() {
                shift = shift.saturating_add(4);
            }
        }
    }
    Some(round_binary(value, sticky, exp.saturating_add(shift)))
}

/// The double nearest to `value × 2^exp` (plus a sliver when `sticky`),
/// ties to even, rounding only once.
fn round_binary(value: u64, sticky: bool, exp: i64) -> f64 {
    if value == 0 {
        return 0.0;
    }
    let bits = 64 - value.leading_zeros() as i64;
    // Exponent of the leading bit.
    let lead = exp.saturating_add(bits - 1);
    if lead > 1023 {
        return f64::INFINITY;
    }
    // Significant bits a double keeps at this magnitude: 53 for normal
    // numbers, fewer for subnormals (the last bit is worth 2^-1074).
    let keep = if lead >= -1022 { 53 } else { lead + 1075 };
    if keep < 0 {
        return 0.0;
    }
    let drop = (bits - keep).max(0) as u32;
    let wide = value as u128;
    let mut kept = (wide >> drop) as u64;
    if drop > 0 {
        let rest = wide & ((1u128 << drop) - 1);
        let half = 1u128 << (drop - 1);
        if rest > half || (rest == half && (sticky || kept & 1 == 1)) {
            kept += 1;
        }
    }
    // `kept × 2^(exp + drop)` is exactly representable (or overflows to
    // infinity after a carry); scale in steps so no step underflows.
    let mut x = kept as f64;
    let mut e = exp + drop as i64;
    while e < -1000 {
        x *= 2f64.powi(-1000);
        e += 1000;
    }
    while e > 1000 {
        x *= 2f64.powi(1000);
        e -= 1000;
    }
    x * 2f64.powi(e as i32)
}

/// `java.util.Random`: the 48-bit linear congruential generator.
#[derive(Debug, Clone)]
pub struct JavaRandom {
    seed: u64,
}

const MULTIPLIER: u64 = 0x5_DEEC_E66D;
const MASK: u64 = (1 << 48) - 1;

impl JavaRandom {
    pub fn new(seed: i64) -> JavaRandom {
        JavaRandom {
            seed: (seed as u64 ^ MULTIPLIER) & MASK,
        }
    }

    fn next(&mut self, bits: u32) -> i32 {
        self.seed = self.seed.wrapping_mul(MULTIPLIER).wrapping_add(0xB) & MASK;
        (self.seed >> (48 - bits)) as i32
    }

    /// `nextInt(bound)`; `None` when `bound` is not positive.
    pub fn next_int(&mut self, bound: i32) -> Option<i32> {
        if bound <= 0 {
            return None;
        }
        let mut r = self.next(31);
        let m = bound - 1;
        if bound & m == 0 {
            return Some(((bound as i64 * r as i64) >> 31) as i32);
        }
        let mut u = r;
        loop {
            r = u % bound;
            if u.wrapping_sub(r).wrapping_add(m) >= 0 {
                return Some(r);
            }
            u = self.next(31);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn double_to_string_matches_java() {
        let cases: &[(f64, &str)] = &[
            (3.0, "3"),
            (2.5, "2.5"),
            (1e7, "1.0E7"),
            (1234567.5, "1234567.5"),
            (0.001, "0.001"),
            (1e-4, "1.0E-4"),
            (0.1 + 0.2, "0.30000000000000004"),
            (123456789012.0, "1.23456789012E11"),
            (1.1f32 as f64, "1.100000023841858"),
            (1.2 + 1.5, "2.7"),
            (-2.5, "-2.5"),
            (-0.0, "-0"),
            (0.0, "0"),
            (f64::NAN, "NaN"),
            (f64::INFINITY, "Infinity"),
            (f64::NEG_INFINITY, "-Infinity"),
            (1e20, "1.0E20"),
            (-1.2345e-10, "-1.2345E-10"),
            (9999999.0, "9999999"),
            (0.00099, "9.9E-4"),
            (100.0, "100"),
            (f64::MAX, "1.7976931348623157E308"),
            (f64::MIN_POSITIVE, "2.2250738585072014E-308"),
            (5e-324, "4.9E-324"),
            (1e23, "1.0E23"),
            (2e-3, "0.002"),
        ];
        for &(x, want) in cases {
            assert_eq!(number_to_string(x), want, "{x:?}");
        }
        assert_eq!(java_double_to_string(3.0), "3.0");
        assert_eq!(java_double_to_string(1e7), "1.0E7");
        assert_eq!(java_float_to_string(1.1), "1.1");
        assert_eq!(java_float_to_string(1e10), "1.0E10");
        assert_eq!(java_float_to_string(1.4e-45), "1.4E-45");
    }

    #[test]
    fn number_format_matches_java() {
        let nf = |x, i, f| number_format(x, i, f, 0, true);
        assert_eq!(nf(12345.6789, 0, 2), "12,345.68");
        assert_eq!(nf(0.125, 1, 2), "0.12");
        assert_eq!(nf(0.135, 1, 2), "0.14");
        assert_eq!(nf(2.0, 2, i32::MAX), "02");
        assert_eq!(nf(10.0 / 7.0, 0, 3), "1.429");
        assert_eq!(nf(1.0, 3, 0), "001");
        assert_eq!(nf(1.0, 4, 0), "0,001");
        assert_eq!(nf(12345.678, 1, 2), "12,345.68");
        assert_eq!(nf(0.5, 0, 2), ".5");
        assert_eq!(nf(0.0, 0, 2), "0");
        assert_eq!(nf(-0.001, 0, 2), "-0");
        assert_eq!(nf(-1234.5, 1, 0), "-1,234");
        assert_eq!(nf(-1235.5, 1, 0), "-1,236");
        assert_eq!(nf(1.005, 1, 2), "1");
        assert_eq!(nf(0.1, 1, 20), "0.1");
        assert_eq!(nf(1e20, 1, 2), "100,000,000,000,000,000,000");
        assert_eq!(nf(999.999, 1, 2), "1,000");
        assert_eq!(nf(f64::NAN, 1, 2), "NaN");
        assert_eq!(nf(f64::NEG_INFINITY, 1, 2), "-\u{221e}");
        assert_eq!(number_format(12345.67, 0, 3, 3, false), "12345.670");
        assert_eq!(number_format(0.0, 0, 2, 2, true), ".00");
        assert_eq!(number_format(1.5, 1, 0, 2, true), "1.50");
    }

    #[test]
    fn parse_double_follows_java() {
        assert_eq!(parse_java_double("12"), Some(12.0));
        assert_eq!(parse_java_double("  -1.5e3 "), Some(-1500.0));
        assert_eq!(parse_java_double("1."), Some(1.0));
        assert_eq!(parse_java_double(".5"), Some(0.5));
        assert_eq!(parse_java_double("1.5f"), Some(1.5));
        assert_eq!(parse_java_double("+Infinity"), Some(f64::INFINITY));
        assert!(parse_java_double("NaN").unwrap().is_nan());
        assert_eq!(parse_java_double("0x1.8p1"), Some(3.0));
        assert_eq!(parse_java_double("1e400"), Some(f64::INFINITY));
        for bad in [
            "", " ", ".", "abc", "1e", "1,5", "inf", "nan", "1.5ff", "0x1.8", "e5",
        ] {
            assert_eq!(parse_java_double(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn hex_exponent_has_one_sign_and_digits() {
        for bad in [
            "0x1p-", "0x1p+", "0x1p", "0x1p--1", "0x1p+-1", "0x1p-+1", "0x1p1a",
        ] {
            assert_eq!(parse_java_double(bad), None, "{bad:?}");
        }
        assert_eq!(parse_java_double("0x1p-1"), Some(0.5));
        assert_eq!(parse_java_double("0x1p+1"), Some(2.0));
        assert_eq!(parse_java_double("0X1P3"), Some(8.0));
    }

    #[test]
    fn hex_exponents_saturate() {
        assert_eq!(parse_java_double("0x1.0p-9223372036854775808"), Some(0.0));
        assert_eq!(
            parse_java_double("0x1.0p9223372036854775807"),
            Some(f64::INFINITY)
        );
        assert_eq!(
            parse_java_double("0x1.0p99999999999999999999999"),
            Some(f64::INFINITY)
        );
        let many = format!("0x{}p-9223372036854775808", "f".repeat(100));
        assert_eq!(parse_java_double(&many), Some(0.0));
        let many = format!("0x{}p0", "f".repeat(300));
        assert_eq!(parse_java_double(&many), Some(f64::INFINITY));
    }

    #[test]
    fn hex_floats_round_correctly() {
        assert_eq!(parse_java_double("0x1.0p-1074"), Some(5e-324));
        assert_eq!(
            number_to_string(parse_java_double("0x1.0p-1074").unwrap()),
            "4.9E-324"
        );
        assert_eq!(parse_java_double("0x1.0p-1075"), Some(0.0));
        assert_eq!(parse_java_double("0x1.8p-1075"), Some(5e-324));
        assert_eq!(parse_java_double("0x1.0p-1022"), Some(f64::MIN_POSITIVE));
        assert_eq!(
            parse_java_double("0x0.8p-1022"),
            Some(f64::MIN_POSITIVE / 2.0)
        );
        assert_eq!(parse_java_double("0x1.fffffffffffffp1023"), Some(f64::MAX));
        assert_eq!(
            parse_java_double("0x1.fffffffffffff8p1023"),
            Some(f64::INFINITY)
        );
        // 2^53 + 1 is a tie: to even (2^53); 2^53 + 3 rounds up.
        assert_eq!(
            parse_java_double("0x20000000000001p0"),
            Some(9007199254740992.0)
        );
        assert_eq!(
            parse_java_double("0x20000000000003p0"),
            Some(9007199254740996.0)
        );
        // ((2^53 + 1) × 2^64 + 1) × 2^-68: a dropped non-zero digit breaks
        // the tie, so it rounds up to (2^53 + 2) / 16.
        assert_eq!(
            parse_java_double("0x200000000000010000000000000001p-68"),
            Some(9007199254740994.0 / 16.0)
        );
        assert_eq!(
            parse_java_double("0x200000000000010000000000000000p-68"),
            Some(9007199254740992.0 / 16.0)
        );
    }

    #[test]
    fn java_random_sequence() {
        let mut r = JavaRandom::new(42);
        let got: Vec<i32> = (0..3).map(|_| 1 + r.next_int(99).unwrap()).collect();
        assert_eq!(got, [63, 49, 64]);
        let mut r = JavaRandom::new(42);
        assert_eq!(r.next_int(10), Some(0));
        // A power-of-two bound takes the high bits: (16 * r) >> 31 = r >> 27.
        let (mut a, mut b) = (JavaRandom::new(7), JavaRandom::new(7));
        assert_eq!(a.next_int(16), Some(b.next(31) >> 27));
        assert_eq!(r.next_int(0), None);
    }
}

#[cfg(test)]
mod java_data {
    use super::*;

    /// `tests/data/java_numbers.tsv`: Java's own output for each value.
    #[test]
    fn number_format_and_to_string_match_jdk_21() {
        let settings = [
            (0, 0, 0, true),
            (1, 2, 0, true),
            (1, 3, 0, false),
            (0, 5, 2, true),
            (1, 10, 0, true),
        ];
        let data = include_str!("../tests/data/java_numbers.tsv");
        let mut rows = 0;
        for line in data.lines().filter(|l| !l.starts_with('#')) {
            let f: Vec<&str> = line.split('\t').collect();
            let d = f64::from_bits(u64::from_str_radix(f[0], 16).unwrap());
            let fl = f32::from_bits(u32::from_str_radix(f[2], 16).unwrap());
            assert_eq!(java_double_to_string(d), f[1], "{line}");
            assert_eq!(java_float_to_string(fl), f[3], "{line}");
            for (i, (min_int, max_frac, min_frac, grouping)) in settings.into_iter().enumerate() {
                let got = number_format(d, min_int, max_frac, min_frac, grouping);
                assert_eq!(got, f[4 + i], "{line}");
            }
            rows += 1;
        }
        assert!(rows > 900);
    }

    /// Deviation D3: where JDK 21 gives 28,551,757,570,297,602,000.
    #[test]
    fn number_format_d3_gives_the_shortest_digits() {
        let x = f64::from_bits(0x43f8_c3c3_83c8_aa8a);
        assert_eq!(java_double_to_string(x), "2.85517575702976E19");
        assert_eq!(
            number_format(x, 0, 0, 0, true),
            "28,551,757,570,297,600,000"
        );
    }
}
