//! Files and folders as whole trees: copy one, compare two, take away whatever sits at a path.

use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::io::Read;
use std::path::Path;

/// Copies a file or a folder, following symlinks inside the source.
pub fn copy(from: &Path, to: &Path) -> Result<()> {
    let m = std::fs::metadata(from).with_context(|| format!("copy source {}", from.display()))?;
    if m.is_file() {
        std::fs::copy(from, to).with_context(|| format!("copying {}", from.display()))?;
        return Ok(());
    }
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)?.flatten() {
        copy(&e.path(), &to.join(e.file_name()))?;
    }
    Ok(())
}

/// Whether `to` is what `copy(from, to)` makes: the same folders and names, the same bytes in
/// each file, nothing more. Symlinks are followed on both sides, as `copy` follows them. Anything
/// unreadable counts as different.
pub fn same(from: &Path, to: &Path) -> bool {
    let (Ok(a), Ok(b)) = (std::fs::metadata(from), std::fs::metadata(to)) else {
        return false;
    };
    if a.is_file() && b.is_file() {
        return a.len() == b.len() && same_bytes(from, to).unwrap_or(false);
    }
    if !(a.is_dir() && b.is_dir()) {
        return false;
    }
    match (names(from), names(to)) {
        (Some(x), Some(y)) if x == y => x.iter().all(|n| same(&from.join(n), &to.join(n))),
        _ => false,
    }
}

/// How many files `copy(from, _)` writes, following symlinks as it does.
pub fn count_files(from: &Path) -> usize {
    match std::fs::metadata(from) {
        Ok(m) if m.is_file() => 1,
        Ok(m) if m.is_dir() => std::fs::read_dir(from)
            .map(|rd| rd.flatten().map(|e| count_files(&e.path())).sum())
            .unwrap_or(0),
        _ => 0,
    }
}

fn names(dir: &Path) -> Option<BTreeSet<OsString>> {
    std::fs::read_dir(dir)
        .ok()?
        .map(|e| e.ok().map(|e| e.file_name()))
        .collect()
}

fn same_bytes(a: &Path, b: &Path) -> std::io::Result<bool> {
    let (mut fa, mut fb) = (std::fs::File::open(a)?, std::fs::File::open(b)?);
    let (mut ba, mut bb) = (vec![0u8; 64 * 1024], vec![0u8; 64 * 1024]);
    loop {
        let (n, m) = (fill(&mut fa, &mut ba)?, fill(&mut fb, &mut bb)?);
        if ba[..n] != bb[..m] {
            return Ok(false);
        }
        if n == 0 {
            return Ok(true);
        }
    }
}

/// Reads until the buffer is full or the file ends, so two files line up chunk by chunk.
fn fill(f: &mut std::fs::File, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut n = 0;
    while n < buf.len() {
        match f.read(&mut buf[n..])? {
            0 => break,
            k => n += k,
        }
    }
    Ok(n)
}

/// Removes a link, a file or a whole folder at `at`, without following a link; nothing there is fine.
pub fn remove(at: &Path) -> Result<()> {
    let r = match std::fs::symlink_metadata(at) {
        Err(_) => return Ok(()),
        Ok(m) if m.is_dir() => std::fs::remove_dir_all(at),
        Ok(_) => std::fs::remove_file(at),
    };
    r.with_context(|| format!("removing {}", at.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_sees_bytes_names_and_extra_entries() {
        let t = tempfile::tempdir().unwrap();
        let src = t.path().join("src");
        std::fs::create_dir_all(src.join("sub")).unwrap();
        std::fs::write(src.join("a.md"), "alpha").unwrap();
        std::fs::write(src.join("sub/b.md"), vec![7u8; 200_000]).unwrap();
        let dst = t.path().join("dst");
        copy(&src, &dst).unwrap();
        assert!(same(&src, &dst));

        let mut big = vec![7u8; 200_000];
        big[150_000] = 8;
        std::fs::write(dst.join("sub/b.md"), &big).unwrap();
        assert!(!same(&src, &dst), "one byte past the first chunk");
        copy(&src, &dst).unwrap();
        assert!(same(&src, &dst));

        std::fs::write(dst.join("extra"), "").unwrap();
        assert!(!same(&src, &dst), "an extra file");
        remove(&dst.join("extra")).unwrap();
        std::fs::write(dst.join("a.md"), "alphA").unwrap();
        assert!(!same(&src, &dst), "same length, other bytes");
        assert!(!same(&src.join("a.md"), &dst), "a file against a folder");
        assert!(!same(&src, &t.path().join("missing")));
    }
}
