//! A tiny interactive shell over the single-node store.
//!
//!   cargo run -- [path/to/log.wal]
//!
//! Commands: PUT <key> <value> | GET <key> | DEL <key> | QUIT

use std::env;
use std::io::{self, BufRead, Write};

use raftkv::KvStore;

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| "kv.wal".to_string());
    let mut store = match KvStore::open(&path) {
        Ok(store) => store,
        Err(e) => {
            eprintln!("failed to open {path}: {e}");
            std::process::exit(1);
        }
    };

    println!("raftkv single-node shell ({path}, {} keys)", store.len());
    println!("commands: PUT <key> <value> | GET <key> | DEL <key> | QUIT");

    let stdin = io::stdin();
    loop {
        print!("> ");
        io::stdout().flush().ok();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {}
            Err(e) => {
                eprintln!("read error: {e}");
                break;
            }
        }

        let mut parts = line.trim().splitn(3, ' ');
        let cmd = parts.next().unwrap_or("").to_ascii_uppercase();
        let arg1 = parts.next();
        let arg2 = parts.next();

        match (cmd.as_str(), arg1, arg2) {
            ("PUT", Some(k), Some(v)) => match store.put(k.as_bytes(), v.as_bytes()) {
                Ok(()) => println!("OK"),
                Err(e) => eprintln!("error: {e}"),
            },
            ("GET", Some(k), None) => match store.get(k.as_bytes()) {
                Some(v) => println!("{}", String::from_utf8_lossy(v)),
                None => println!("(nil)"),
            },
            ("DEL", Some(k), None) => match store.delete(k.as_bytes()) {
                Ok(()) => println!("OK"),
                Err(e) => eprintln!("error: {e}"),
            },
            ("QUIT", _, _) | ("EXIT", _, _) => break,
            ("", _, _) => {}
            _ => println!("usage: PUT <key> <value> | GET <key> | DEL <key> | QUIT"),
        }
    }
}
