//! A minimal write-ahead log (WAL) of opaque byte records
//!
//! On-disk record format (little endian):
//!
//!   [ payload_len: u32 ][ crc32(payload): u32 ][ payload bytes ... ]
//!
//! Recovery rule: on open, records are read front to back. The first record
//! that is incomplete ("torn write" from a crash) or fails its checksum ends
//! the log. Everything from that point on is truncated away, so new appends
//! always start at a clean record boundary.

use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

const HEADER_LEN: usize = 8;
/// Sanity cap so a corrupt length field cannot treat garbage as a record
const MAX_RECORD_LEN: u32 = 64 * 1024 * 1024;

pub struct Wal {
    file: File,
    sync_on_append: bool,
}

impl Wal {
    /// Opens (or creates) the log at `path` and returns it together with every
    /// valid record found, in order. Any torn or corrupt tail is truncated.
    ///
    /// With `sync_on_append`, each append is flushed to disk (`fsync`) before
    /// returning, which is what makes an acknowledged write durable. It is also
    /// the main throughput limiter; batching writes to be done later!
    pub fn open<P: AsRef<Path>>(path: P, sync_on_append: bool) -> io::Result<(Wal, Vec<Vec<u8>>)> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;

        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let mut records = Vec::new();
        let mut offset = 0usize;
        while let Some((payload, next)) = parse_record(&data, offset) {
            records.push(payload.to_vec());
            offset = next;
        }

        if offset < data.len() {
            // Torn write or corruption at the tail: discard it
            file.set_len(offset as u64)?;
            file.sync_all()?;
        }
        file.seek(SeekFrom::Start(offset as u64))?;

        Ok((
            Wal {
                file,
                sync_on_append,
            },
            records,
        ))
    }

    /// Appends one record. The header and payload go out in a single write call
    /// to keep the window for a torn header as small as possible.
    pub fn append(&mut self, payload: &[u8]) -> io::Result<()> {
        let len = u32::try_from(payload.len())
            .ok()
            .filter(|&l| l <= MAX_RECORD_LEN)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "record too large"))?;

        let mut buf = Vec::with_capacity(HEADER_LEN + payload.len());
        buf.extend_from_slice(&len.to_le_bytes());
        buf.extend_from_slice(&crc32(payload).to_le_bytes());
        buf.extend_from_slice(payload);

        self.file.write_all(&buf)?;
        if self.sync_on_append {
            self.file.sync_data()?;
        }
        Ok(())
    }
}

/// Parses the record starting at `offset`. Returns the payload and the offset of
/// the next record, or `None` if the record is incomplete or fails validation.
fn parse_record(data: &[u8], offset: usize) -> Option<(&[u8], usize)> {
    let header = data.get(offset..offset + HEADER_LEN)?;
    let len = u32::from_le_bytes(header[0..4].try_into().unwrap());
    let crc = u32::from_le_bytes(header[4..8].try_into().unwrap());
    if len > MAX_RECORD_LEN {
        return None;
    }
    let start = offset + HEADER_LEN;
    let end = start.checked_add(len as usize)?;
    let payload = data.get(start..end)?;
    if crc32(payload) != crc {
        return None;
    }
    Some((payload, end))
}

/// CRC-32 (IEEE 802.3, the same polynomial as zlib/gzip), bitwise implementation.
/// Slow but tiny and dependency-free; swap for a table-driven version or a crate
/// if profiling ever shows it matters.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 == 1 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    struct TempFile(PathBuf);

    impl TempFile {
        fn new(name: &str) -> Self {
            let mut p = std::env::temp_dir();
            p.push(format!("raftkv-unit-{}-{}.wal", name, std::process::id()));
            let _ = fs::remove_file(&p);
            TempFile(p)
        }
    }

    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn crc32_matches_known_check_value() {
        // The standard CRC-32 check value for the ASCII string "123456789".
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn records_round_trip_in_order() {
        let tmp = TempFile::new("roundtrip");
        {
            let (mut wal, existing) = Wal::open(&tmp.0, false).unwrap();
            assert!(existing.is_empty());
            wal.append(b"one").unwrap();
            wal.append(b"").unwrap();
            wal.append(b"three").unwrap();
        }
        let (_wal, records) = Wal::open(&tmp.0, false).unwrap();
        assert_eq!(
            records,
            vec![b"one".to_vec(), Vec::new(), b"three".to_vec()]
        );
    }
}
