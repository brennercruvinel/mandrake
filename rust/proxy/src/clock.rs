//! Timestamps and request ids, without a date crate.

use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// RFC 3339 in UTC with milliseconds, e.g. `2026-10-24T10:11:12.345Z`.
pub fn rfc3339(t: SystemTime) -> String {
    let d = t.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = d.as_secs();
    let (y, m, day) = civil(secs.div_euclid(86_400) as i64);
    let s = secs % 86_400;
    format!(
        "{y:04}-{m:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        s / 3600,
        s / 60 % 60,
        s % 60,
        d.subsec_millis()
    )
}

pub fn now() -> String {
    rfc3339(SystemTime::now())
}

/// Unique per process run and sortable by time: `<unix ms>-<pid>-<n>`. Safe as a file name.
pub fn new_id() -> String {
    static N: AtomicU32 = AtomicU32::new(0);
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let n = N.fetch_add(1, Ordering::Relaxed);
    format!("{ms:013}-{:x}-{n:04}", std::process::id())
}

/// Days since 1970-01-01 to (year, month, day), after Howard Hinnant's `civil_from_days`.
fn civil(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn formats_known_instants() {
        assert_eq!(rfc3339(UNIX_EPOCH), "1970-01-01T00:00:00.000Z");
        let t = UNIX_EPOCH + Duration::from_millis(1_709_164_800_123); // leap day 2024
        assert_eq!(rfc3339(t), "2024-02-29T00:00:00.123Z");
        let t = UNIX_EPOCH + Duration::from_secs(1_793_016_672);
        assert_eq!(rfc3339(t), "2026-10-26T12:11:12.000Z");
    }

    #[test]
    fn ids_are_unique() {
        assert_ne!(new_id(), new_id());
    }
}
