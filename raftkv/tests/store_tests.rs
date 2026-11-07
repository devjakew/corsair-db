//! Integration tests for the single-node store, focused on durability and crash recovery

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use raftkv::KvStore;

/// Unique temp file path that is cleaned up when dropped
struct TempWal(PathBuf);

impl TempWal {
    fn new(name: &str) -> Self {
        let mut p = std::env::temp_dir();
        p.push(format!("raftkv-it-{}-{}.wal", name, std::process::id()));
        let _ = fs::remove_file(&p);
        TempWal(p)
    }
}

impl Drop for TempWal {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[test]
fn put_get_delete() {
    let wal = TempWal::new("basic");
    let mut s = KvStore::open(&wal.0).unwrap(); // default: fsync on every write
    assert_eq!(s.get(b"a"), None);

    s.put(b"a", b"1").unwrap();
    assert_eq!(s.get(b"a"), Some(&b"1"[..]));

    s.delete(b"a").unwrap();
    assert_eq!(s.get(b"a"), None);
    assert!(s.is_empty());
}

#[test]
fn overwrite_keeps_latest_value() {
    let wal = TempWal::new("overwrite");
    let mut s = KvStore::open_with(&wal.0, false).unwrap();
    s.put(b"k", b"old").unwrap();
    s.put(b"k", b"new").unwrap();
    assert_eq!(s.get(b"k"), Some(&b"new"[..]));
    assert_eq!(s.len(), 1);
}

#[test]
fn data_survives_reopen() {
    let wal = TempWal::new("reopen");
    {
        let mut s = KvStore::open_with(&wal.0, false).unwrap();
        s.put(b"a", b"1").unwrap();
        s.put(b"b", b"2").unwrap();
        s.delete(b"a").unwrap();
    }
    let s = KvStore::open_with(&wal.0, false).unwrap();
    assert_eq!(s.get(b"a"), None);
    assert_eq!(s.get(b"b"), Some(&b"2"[..]));
}

#[test]
fn torn_tail_is_discarded_and_log_stays_usable() {
    let wal = TempWal::new("torn");
    {
        let mut s = KvStore::open_with(&wal.0, false).unwrap();
        s.put(b"a", b"1").unwrap();
        s.put(b"b", b"2").unwrap();
    }
    let good_len = fs::metadata(&wal.0).unwrap().len();

    // Simulate a crash mid write: header promising 100 bytes, then only 3
    {
        let mut f = OpenOptions::new().append(true).open(&wal.0).unwrap();
        f.write_all(&100u32.to_le_bytes()).unwrap();
        f.write_all(&0u32.to_le_bytes()).unwrap();
        f.write_all(b"abc").unwrap();
    }

    {
        let mut s = KvStore::open_with(&wal.0, false).unwrap();
        assert_eq!(s.get(b"a"), Some(&b"1"[..]));
        assert_eq!(s.get(b"b"), Some(&b"2"[..]));
        // garbage tail was truncated away on open
        assert_eq!(fs::metadata(&wal.0).unwrap().len(), good_len);
        // new writes land cleanly after the last good record
        s.put(b"c", b"3").unwrap();
    }

    let s = KvStore::open_with(&wal.0, false).unwrap();
    assert_eq!(s.get(b"c"), Some(&b"3"[..]));
    assert_eq!(s.len(), 3);
}

#[test]
fn corrupt_checksum_drops_that_record() {
    let wal = TempWal::new("crc");
    {
        let mut s = KvStore::open_with(&wal.0, false).unwrap();
        s.put(b"a", b"1").unwrap();
        s.put(b"b", b"2").unwrap();
    }

    // flip a bit in the last byte of the final record's payload
    let mut bytes = fs::read(&wal.0).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xFF;
    fs::write(&wal.0, &bytes).unwrap();

    let s = KvStore::open_with(&wal.0, false).unwrap();
    assert_eq!(s.get(b"a"), Some(&b"1"[..]));
    assert_eq!(s.get(b"b"), None);
}

#[test]
fn log_cut_short_mid_record_recovers_earlier_records() {
    let wal = TempWal::new("cut");
    {
        let mut s = KvStore::open_with(&wal.0, false).unwrap();
        s.put(b"a", b"1").unwrap();
        s.put(b"b", b"2").unwrap();
    }

    let mut bytes = fs::read(&wal.0).unwrap();
    bytes.truncate(bytes.len() - 3);
    fs::write(&wal.0, &bytes).unwrap();

    let s = KvStore::open_with(&wal.0, false).unwrap();
    assert_eq!(s.get(b"a"), Some(&b"1"[..]));
    assert_eq!(s.get(b"b"), None);
}
