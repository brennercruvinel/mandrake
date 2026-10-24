//! Numbers, sizes and times written the way the labels are: Brazilian Portuguese.

/// `12345` → `12.345`.
pub fn num(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

/// Signed [`num`], with an explicit `+` for gains.
pub fn signed(n: i64) -> String {
    match n.cmp(&0) {
        std::cmp::Ordering::Less => format!("-{}", num(n.unsigned_abs())),
        std::cmp::Ordering::Equal => "0".into(),
        std::cmp::Ordering::Greater => format!("+{}", num(n as u64)),
    }
}

/// `0.2345` → `23,4%`.
pub fn pct(ratio: f64) -> String {
    format!("{:.1}%", ratio * 100.0).replace('.', ",")
}

/// `1536` → `1,5 KB`; plain bytes under 1 KiB.
pub fn bytes(n: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    if n < 1024 {
        return format!("{n} B");
    }
    let mut v = n as f64;
    let mut unit = 0;
    while v >= 1024.0 && unit < UNITS.len() - 1 {
        v /= 1024.0;
        unit += 1;
    }
    format!("{v:.1} {}", UNITS[unit]).replace('.', ",")
}

/// Share removed going from `before` to `after`, 0 when there was nothing before.
pub fn saved_ratio(before: u64, after: u64) -> f64 {
    if before == 0 {
        0.0
    } else {
        1.0 - after as f64 / before as f64
    }
}

/// The time of day of an RFC 3339 stamp, `2026-10-24T13:05:09.120Z` → `13:05:09`.
pub fn clock(ts: &str) -> &str {
    ts.split_once('T')
        .map(|(_, t)| t.get(..8).unwrap_or(t))
        .unwrap_or(ts)
}

/// `claude-sonnet-4-5-20250929` → `sonnet-4-5`: the family and version, without the vendor or date.
pub fn model(name: Option<&str>) -> String {
    let Some(name) = name else { return "-".into() };
    let name = name.strip_prefix("claude-").unwrap_or(name);
    match name.rsplit_once('-') {
        Some((head, date)) if date.len() == 8 && date.bytes().all(|b| b.is_ascii_digit()) => {
            head.to_string()
        }
        _ => name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_like_pt_br() {
        assert_eq!(num(0), "0");
        assert_eq!(num(999), "999");
        assert_eq!(num(1234567), "1.234.567");
        assert_eq!(signed(-1500), "-1.500");
        assert_eq!(signed(20), "+20");
        assert_eq!(pct(0.2345), "23,4%");
        assert_eq!(bytes(512), "512 B");
        assert_eq!(bytes(1536), "1,5 KB");
        assert_eq!(clock("2026-10-24T13:05:09.120Z"), "13:05:09");
        assert_eq!(model(Some("claude-sonnet-4-5-20250929")), "sonnet-4-5");
        assert_eq!(model(Some("claude-opus-4-1")), "opus-4-1");
        assert_eq!(model(None), "-");
        assert!((saved_ratio(200, 150) - 0.25).abs() < 1e-9);
        assert_eq!(saved_ratio(0, 10), 0.0);
    }
}
