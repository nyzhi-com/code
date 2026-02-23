use std::fmt::Write;

/// Generate a unified diff between `old` and `new` content for the given file path.
/// Returns an empty string if the contents are identical.
pub fn unified_diff(path: &str, old: &str, new: &str, context_lines: usize) -> String {
    if old == new {
        return String::new();
    }

    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let hunks = compute_hunks(&old_lines, &new_lines, context_lines);
    if hunks.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    let _ = writeln!(out, "--- a/{path}");
    let _ = writeln!(out, "+++ b/{path}");

    for hunk in &hunks {
        let _ = writeln!(
            out,
            "@@ -{},{} +{},{} @@",
            hunk.old_start + 1,
            hunk.old_count,
            hunk.new_start + 1,
            hunk.new_count,
        );
        for line in &hunk.lines {
            let _ = writeln!(out, "{line}");
        }
    }

    out
}

/// Truncate a diff to at most `max_lines` output lines, appending a note if truncated.
pub fn truncate_diff(diff: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = diff.lines().collect();
    if lines.len() <= max_lines {
        return diff.to_string();
    }
    let mut out: String = lines[..max_lines].join("\n");
    let _ = write!(out, "\n... ({} more lines)", lines.len() - max_lines);
    out
}

struct Hunk {
    old_start: usize,
    old_count: usize,
    new_start: usize,
    new_count: usize,
    lines: Vec<String>,
}

fn compute_hunks(old: &[&str], new: &[&str], ctx: usize) -> Vec<Hunk> {
    let lcs = lcs_table(old, new);
    let edits = backtrack(&lcs, old, new);

    let mut raw_changes: Vec<(usize, usize, EditOp)> = Vec::new();
    let mut oi = 0usize;
    let mut ni = 0usize;
    for op in &edits {
        match op {
            EditOp::Equal => {
                oi += 1;
                ni += 1;
            }
            EditOp::Delete => {
                raw_changes.push((oi, ni, EditOp::Delete));
                oi += 1;
            }
            EditOp::Insert => {
                raw_changes.push((oi, ni, EditOp::Insert));
                ni += 1;
            }
        }
    }

    if raw_changes.is_empty() {
        return Vec::new();
    }

    // Group changes into hunks with context
    let mut hunks = Vec::new();
    let mut i = 0;
    while i < raw_changes.len() {
        let first = &raw_changes[i];
        let hunk_old_start = first.0.saturating_sub(ctx);
        let hunk_new_start = first.1.saturating_sub(ctx);

        // Find end of this hunk (merge nearby changes)
        let mut j = i;
        while j + 1 < raw_changes.len() {
            let gap_old = raw_changes[j + 1].0.saturating_sub(raw_changes[j].0);
            let gap_new = raw_changes[j + 1].1.saturating_sub(raw_changes[j].1);
            let gap = gap_old.max(gap_new);
            if gap <= ctx * 2 + 1 {
                j += 1;
            } else {
                break;
            }
        }

        let last = &raw_changes[j];
        let hunk_old_end = (last.0 + 1).min(old.len());
        let hunk_new_end = (last.1 + 1).min(new.len());
        let old_end_ctx = (hunk_old_end + ctx).min(old.len());
        let new_end_ctx = (hunk_new_end + ctx).min(new.len());

        let mut lines = Vec::new();
        let mut ho = hunk_old_start;
        #[allow(unused_assignments)]
        let mut hn = hunk_new_start;
        let mut edit_idx = i;
        loop {
            if ho >= old_end_ctx && hn >= new_end_ctx {
                break;
            }

            if edit_idx <= j && edit_idx < raw_changes.len() {
                let (co, cn, ref op) = raw_changes[edit_idx];
                // Emit context lines before this change
                while ho < co && ho < old_end_ctx {
                    lines.push(format!(" {}", old[ho]));
                    ho += 1;
                    hn += 1;
                }
                match op {
                    EditOp::Delete => {
                        lines.push(format!("-{}", old[co]));
                        ho += 1;
                        edit_idx += 1;
                    }
                    EditOp::Insert => {
                        lines.push(format!("+{}", new[cn]));
                        hn += 1;
                        edit_idx += 1;
                    }
                    EditOp::Equal => {
                        edit_idx += 1;
                    }
                }
            } else {
                // Trailing context
                if ho < old_end_ctx {
                    lines.push(format!(" {}", old[ho]));
                    ho += 1;
                    hn += 1;
                } else {
                    break;
                }
            }
        }

        hunks.push(Hunk {
            old_start: hunk_old_start,
            old_count: old_end_ctx - hunk_old_start,
            new_start: hunk_new_start,
            new_count: new_end_ctx - hunk_new_start,
            lines,
        });

        i = j + 1;
    }

    hunks
}

#[derive(Clone)]
enum EditOp {
    Equal,
    Delete,
    Insert,
}

fn lcs_table(old: &[&str], new: &[&str]) -> Vec<Vec<usize>> {
    let m = old.len();
    let n = new.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if old[i - 1] == new[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }
    dp
}

fn backtrack(dp: &[Vec<usize>], old: &[&str], new: &[&str]) -> Vec<EditOp> {
    let mut ops = Vec::new();
    let mut i = old.len();
    let mut j = new.len();

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old[i - 1] == new[j - 1] {
            ops.push(EditOp::Equal);
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || dp[i][j - 1] >= dp[i - 1][j]) {
            ops.push(EditOp::Insert);
            j -= 1;
        } else {
            ops.push(EditOp::Delete);
            i -= 1;
        }
    }

    ops.reverse();
    ops
}
