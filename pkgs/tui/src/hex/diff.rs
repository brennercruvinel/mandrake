//! Which bytes of the "before" body are gone and which bytes of the "after" body are new.
//!
//! Myers over bytes gives the exact picture; when it cannot finish within the budget, the bodies are
//! cut into JSON lines (after every newline or comma, since request bodies usually arrive minified)
//! and diffed by line, which is coarser but always fast.

use similar::{Algorithm, DiffOp, DiffTag, capture_diff_slices_deadline};
use std::ops::Range;
use std::time::{Duration, Instant};

/// How long the byte diff may take before falling back to lines.
pub const BUDGET: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Bytes,
    /// The byte diff ran out of time.
    Lines,
}

/// One stretch where the bodies differ, as byte ranges of each.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    pub before: Range<usize>,
    pub after: Range<usize>,
}

/// An op as byte ranges of each body.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Op {
    tag: DiffTag,
    before: Range<usize>,
    after: Range<usize>,
}

#[derive(Debug, Clone)]
pub struct Diff {
    pub method: Method,
    /// Per byte of "before": true when "after" does not have it.
    pub removed: Vec<bool>,
    /// Per byte of "after": true when "before" did not have it.
    pub inserted: Vec<bool>,
    pub hunks: Vec<Hunk>,
    ops: Vec<Op>,
    len_before: usize,
    len_after: usize,
}

/// Which body a position belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Body {
    Before,
    After,
}

impl Body {
    pub fn other(self) -> Body {
        match self {
            Body::Before => Body::After,
            Body::After => Body::Before,
        }
    }
}

pub fn diff(before: &[u8], after: &[u8], budget: Duration) -> Diff {
    let deadline = Instant::now() + budget;
    let ops = capture_diff_slices_deadline(Algorithm::Myers, before, after, Some(deadline));
    if Instant::now() < deadline {
        let ops = ops
            .iter()
            .map(|op| as_bytes(op, &[0], &[0], true))
            .collect();
        return Diff::from_ops(Method::Bytes, ops, before.len(), after.len());
    }
    let (lb, la) = (lines(before), lines(after));
    let (sb, sa) = (slices(before, &lb), slices(after, &la));
    // A second budget: if even lines run out, similar still returns a valid, coarser diff.
    let ops =
        capture_diff_slices_deadline(Algorithm::Myers, &sb, &sa, Some(Instant::now() + budget));
    let ops = ops.iter().map(|op| as_bytes(op, &lb, &la, false)).collect();
    Diff::from_ops(Method::Lines, ops, before.len(), after.len())
}

/// Start offsets of each JSON line, plus the total length at the end.
fn lines(body: &[u8]) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, b) in body.iter().enumerate() {
        if matches!(b, b'\n' | b',') && i + 1 < body.len() {
            starts.push(i + 1);
        }
    }
    starts.push(body.len());
    if body.is_empty() {
        starts.truncate(1);
    }
    starts
}

fn slices<'a>(body: &'a [u8], starts: &[usize]) -> Vec<&'a [u8]> {
    starts.windows(2).map(|w| &body[w[0]..w[1]]).collect()
}

/// Turns an op over units (bytes, or lines whose starts are given) into byte ranges.
fn as_bytes(op: &DiffOp, lb: &[usize], la: &[usize], units_are_bytes: bool) -> Op {
    let (tag, b, a) = op.as_tag_tuple();
    let conv = |r: Range<usize>, starts: &[usize]| {
        if units_are_bytes {
            r
        } else {
            starts[r.start]..starts[r.end]
        }
    };
    Op {
        tag,
        before: conv(b, lb),
        after: conv(a, la),
    }
}

impl Diff {
    fn from_ops(method: Method, ops: Vec<Op>, len_before: usize, len_after: usize) -> Self {
        let mut removed = vec![false; len_before];
        let mut inserted = vec![false; len_after];
        let mut hunks: Vec<Hunk> = Vec::new();
        for op in &ops {
            if op.tag == DiffTag::Equal {
                continue;
            }
            removed[op.before.clone()].fill(true);
            inserted[op.after.clone()].fill(true);
            // Adjacent changes (a delete right before an insert) read as one stretch.
            match hunks.last_mut() {
                Some(h) if h.before.end == op.before.start && h.after.end == op.after.start => {
                    h.before.end = op.before.end;
                    h.after.end = op.after.end;
                }
                _ => hunks.push(Hunk {
                    before: op.before.clone(),
                    after: op.after.clone(),
                }),
            }
        }
        Self {
            method,
            removed,
            inserted,
            hunks,
            ops,
            len_before,
            len_after,
        }
    }

    pub fn removed_bytes(&self) -> usize {
        self.removed.iter().filter(|&&r| r).count()
    }

    pub fn inserted_bytes(&self) -> usize {
        self.inserted.iter().filter(|&&r| r).count()
    }

    pub fn len(&self, body: Body) -> usize {
        match body {
            Body::Before => self.len_before,
            Body::After => self.len_after,
        }
    }

    /// The position in the other body that lines up with `pos` in `from`: the same byte where they
    /// agree, the start of the counterpart where they differ.
    pub fn map(&self, pos: usize, from: Body) -> usize {
        let range = |op: &Op, b: Body| match b {
            Body::Before => op.before.clone(),
            Body::After => op.after.clone(),
        };
        let i = self.ops.partition_point(|op| range(op, from).end <= pos);
        let Some(op) = self.ops.get(i) else {
            return self.len(from.other());
        };
        let (src, dst) = (range(op, from), range(op, from.other()));
        let into = pos.saturating_sub(src.start);
        match op.tag {
            DiffTag::Equal => dst.start + into,
            _ => dst.start + into.min(dst.len()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_diff_marks_the_cut() {
        let before = br#"{"a":1,"system":"cut me","b":2}"#;
        let after = br#"{"a":1,"b":2}"#;
        let d = diff(before, after, BUDGET);
        assert_eq!(d.method, Method::Bytes);
        assert_eq!(d.removed_bytes(), before.len() - after.len());
        assert_eq!(d.inserted_bytes(), 0);
        assert_eq!(d.hunks.len(), 1);
        let cut = &before[d.hunks[0].before.clone()];
        assert!(String::from_utf8_lossy(cut).contains("cut me"));
        // Past the cut, both sides line up on the same `b":2}`; the quote before it is matched by
        // the prefix, so the cut runs from `system` to the quote that opens `"b"`.
        let b_pos = before.len() - 5;
        assert_eq!(&after[d.map(b_pos, Body::Before)..], &before[b_pos..]);
        assert_eq!(d.map(before.len(), Body::Before), after.len());
    }

    #[test]
    fn no_budget_falls_back_to_lines() {
        let before = b"{\"x\":[1,2,3],\"drop\":\"zzz\",\"y\":4}";
        let after = b"{\"x\":[1,2,3],\"y\":4}";
        let d = diff(before, after, Duration::ZERO);
        assert_eq!(d.method, Method::Lines);
        assert_eq!(d.hunks.len(), 1);
        assert_eq!(&before[d.hunks[0].before.clone()], b"\"drop\":\"zzz\",");
        assert_eq!(d.inserted_bytes(), 0);
    }

    #[test]
    fn insertions_and_empty_bodies() {
        let d = diff(b"", b"new", BUDGET);
        assert_eq!(d.inserted_bytes(), 3);
        assert_eq!(d.map(0, Body::Before), 3);
        let d = diff(b"same", b"same", BUDGET);
        assert!(d.hunks.is_empty());
        assert_eq!(d.map(2, Body::After), 2);
    }
}
