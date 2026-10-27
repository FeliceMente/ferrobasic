//! A line diff for the report: what the recording says against what we
//! expected by hand.

/// The changed lines of `old` → `new`, `-` and `+` with two lines of
/// context around each change, `…` between distant changes. Empty when
/// they are equal.
pub fn diff(old: &str, new: &str) -> String {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();
    // Longest common subsequence, from the end.
    let mut lcs = vec![vec![0u32; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            lcs[i][j] = if a[i] == b[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }
    let mut ops: Vec<(char, &str)> = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < a.len() || j < b.len() {
        if i < a.len() && j < b.len() && a[i] == b[j] {
            ops.push((' ', a[i]));
            i += 1;
            j += 1;
        } else if i < a.len() && (j == b.len() || lcs[i + 1][j] >= lcs[i][j + 1]) {
            ops.push(('-', a[i]));
            i += 1;
        } else {
            ops.push(('+', b[j]));
            j += 1;
        }
    }
    if ops.iter().all(|(op, _)| *op == ' ') && old.ends_with('\n') == new.ends_with('\n') {
        return String::new();
    }
    let near = |k: usize| {
        ops[k.saturating_sub(2)..(k + 3).min(ops.len())]
            .iter()
            .any(|(op, _)| *op != ' ')
    };
    let mut out = String::new();
    let mut skipped = false;
    for (k, (op, line)) in ops.iter().enumerate() {
        if near(k) {
            if skipped {
                out.push_str("…\n");
                skipped = false;
            }
            out.push_str(&format!("{op} {line}\n"));
        } else {
            skipped = true;
        }
    }
    if old.ends_with('\n') != new.ends_with('\n') {
        out.push_str("(the final line end differs)\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_texts_have_no_diff() {
        assert_eq!(diff("a\nb\n", "a\nb\n"), "");
        assert_eq!(diff("", ""), "");
    }

    #[test]
    fn shows_changes_with_context() {
        let old = "1\n2\n3\n4\n5\n6\n7\n8\n9\n";
        let new = "1\n2\n3\n4\nfive\n6\n7\n8\n9\nten\n";
        assert_eq!(
            diff(old, new),
            "…\n  3\n  4\n- 5\n+ five\n  6\n  7\n  8\n  9\n+ ten\n"
        );
        assert_eq!(diff("a", "a\n"), "(the final line end differs)\n");
        let far = diff("x\n1\n2\n3\n4\n5\n6\ny\n", "1\n2\n3\n4\n5\n6\n");
        assert_eq!(far, "- x\n  1\n  2\n…\n  5\n  6\n- y\n");
    }
}
