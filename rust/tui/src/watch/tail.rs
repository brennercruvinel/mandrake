//! Reads only what was appended to `events.jsonl` since the last look.

use anyhow::Result;
use mandrake_meter::Event;
use std::fs::File;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::path::PathBuf;

/// What one look at the file found.
#[derive(Debug, Default)]
pub struct Polled {
    /// The file shrank or was replaced; whatever was read before no longer stands.
    pub reset: bool,
    pub events: Vec<Event>,
}

/// A byte offset into the events file; it only moves past complete lines, so a torn last write is
/// read again whole on the next poll.
#[derive(Debug)]
pub struct Tail {
    path: PathBuf,
    offset: u64,
}

impl Tail {
    pub fn new(path: PathBuf) -> Self {
        Self { path, offset: 0 }
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub fn poll(&mut self) -> Result<Polled> {
        let mut f = match File::open(&self.path) {
            Ok(f) => f,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                let reset = self.offset > 0;
                self.offset = 0;
                return Ok(Polled {
                    reset,
                    events: vec![],
                });
            }
            Err(e) => return Err(e.into()),
        };
        let len = f.metadata()?.len();
        let mut out = Polled::default();
        if len < self.offset {
            out.reset = true;
            self.offset = 0;
        }
        if len == self.offset {
            return Ok(out);
        }
        f.seek(SeekFrom::Start(self.offset))?;
        let mut buf = Vec::with_capacity((len - self.offset) as usize);
        f.take(len - self.offset).read_to_end(&mut buf)?;
        let Some(last_newline) = buf.iter().rposition(|&b| b == b'\n') else {
            return Ok(out);
        };
        let complete = &buf[..=last_newline];
        self.offset += complete.len() as u64;
        out.events = complete
            .split(|&b| b == b'\n')
            .filter(|l| !l.is_empty())
            .filter_map(|l| serde_json::from_slice(l).ok())
            .collect();
        Ok(out)
    }
}
