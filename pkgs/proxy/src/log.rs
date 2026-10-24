//! The proxy's text log: one line per request, no headers and no bodies, rotated by size.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const MAX_BYTES: u64 = 5 << 20;
pub const KEEP: u32 = 3;

/// `proxy.log`, then `proxy.log.1` (newest) to `proxy.log.<keep>` (oldest).
#[derive(Debug)]
pub struct RotatingLog {
    path: PathBuf,
    max: u64,
    keep: u32,
}

impl RotatingLog {
    pub fn new(path: PathBuf) -> Self {
        Self::with_limits(path, MAX_BYTES, KEEP)
    }

    pub fn with_limits(path: PathBuf, max: u64, keep: u32) -> Self {
        Self { path, max, keep }
    }

    /// Appends one line. Logging never fails a request, so errors are dropped.
    pub fn line(&self, text: &str) {
        let _ = self.try_line(text);
    }

    fn try_line(&self, text: &str) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let size = std::fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0);
        if size > 0 && size + text.len() as u64 + 1 > self.max {
            self.rotate()?;
        }
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        f.write_all(format!("{text}\n").as_bytes())
    }

    fn rotate(&self) -> std::io::Result<()> {
        let nth = |n: u32| numbered(&self.path, n);
        let _ = std::fs::remove_file(nth(self.keep));
        for n in (1..self.keep).rev() {
            let from = nth(n);
            if from.exists() {
                std::fs::rename(&from, nth(n + 1))?;
            }
        }
        if self.keep == 0 {
            return std::fs::remove_file(&self.path);
        }
        std::fs::rename(&self.path, nth(1))
    }
}

fn numbered(path: &Path, n: u32) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(format!(".{n}"));
    PathBuf::from(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotates_and_keeps_three() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("logs/proxy.log");
        let log = RotatingLog::with_limits(path.clone(), 20, 3);
        for i in 0..6 {
            log.line(&format!("line {i} 0123456"));
        }
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "line 5 0123456\n");
        assert_eq!(
            std::fs::read_to_string(numbered(&path, 1)).unwrap(),
            "line 4 0123456\n"
        );
        assert_eq!(
            std::fs::read_to_string(numbered(&path, 3)).unwrap(),
            "line 2 0123456\n"
        );
        assert!(!numbered(&path, 4).exists());
    }
}
