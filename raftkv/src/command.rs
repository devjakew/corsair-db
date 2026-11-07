//! Commands are the unit of change for the state machine.
//!
//! Today a command is appended to the local write-ahead log and then applied.
//! Once Raft lands, the exact same encoded bytes become the payload of a Raft
//! log entry, so the state machine never needs to know where a command came from.
//!
//! Encoding (little endian):
//!   Put:    [1][key_len: u32][key][value_len: u32][value]
//!   Delete: [2][key_len: u32][key]

use std::fmt;

const OP_PUT: u8 = 1;
const OP_DELETE: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Put { key: Vec<u8>, value: Vec<u8> },
    Delete { key: Vec<u8> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// Buffer ended before the command was complete
    Truncated,
    /// First byte was not a known operation code
    UnknownOp(u8),
    /// Extra bytes remained after a complete command
    TrailingBytes,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::Truncated => write!(f, "truncated command"),
            DecodeError::UnknownOp(op) => write!(f, "unknown op code {op}"),
            DecodeError::TrailingBytes => write!(f, "trailing bytes after command"),
        }
    }
}

impl std::error::Error for DecodeError {}

impl Command {
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        match self {
            Command::Put { key, value } => {
                buf.push(OP_PUT);
                put_bytes(&mut buf, key);
                put_bytes(&mut buf, value);
            }
            Command::Delete { key } => {
                buf.push(OP_DELETE);
                put_bytes(&mut buf, key);
            }
        }
        buf
    }

    pub fn decode(data: &[u8]) -> Result<Command, DecodeError> {
        let (&op, mut rest) = data.split_first().ok_or(DecodeError::Truncated)?;
        let cmd = match op {
            OP_PUT => {
                let key = take_bytes(&mut rest)?;
                let value = take_bytes(&mut rest)?;
                Command::Put { key, value }
            }
            OP_DELETE => {
                let key = take_bytes(&mut rest)?;
                Command::Delete { key }
            }
            other => return Err(DecodeError::UnknownOp(other)),
        };
        if !rest.is_empty() {
            return Err(DecodeError::TrailingBytes);
        }
        Ok(cmd)
    }
}

fn put_bytes(buf: &mut Vec<u8>, bytes: &[u8]) {
    let len = u32::try_from(bytes.len()).expect("key/value larger than 4 GiB");
    buf.extend_from_slice(&len.to_le_bytes());
    buf.extend_from_slice(bytes);
}

fn take_bytes(rest: &mut &[u8]) -> Result<Vec<u8>, DecodeError> {
    if rest.len() < 4 {
        return Err(DecodeError::Truncated);
    }
    let (len_bytes, tail) = rest.split_at(4);
    let len = u32::from_le_bytes(len_bytes.try_into().unwrap()) as usize;
    if tail.len() < len {
        return Err(DecodeError::Truncated);
    }
    let (body, tail) = tail.split_at(len);
    *rest = tail;
    Ok(body.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_round_trips() {
        let cmd = Command::Put {
            key: b"k".to_vec(),
            value: b"v".to_vec(),
        };
        let encoded = cmd.encode();
        assert_eq!(Command::decode(&encoded), Ok(cmd));
    }

    #[test]
    fn delete_round_trips() {
        let cmd = Command::Delete { key: b"k".to_vec() };
        let encoded = cmd.encode();
        assert_eq!(Command::decode(&encoded), Ok(cmd));
    }

    #[test]
    fn empty_key_and_value_round_trip() {
        let cmd = Command::Put {
            key: Vec::new(),
            value: Vec::new(),
        };
        let encoded = cmd.encode();
        assert_eq!(Command::decode(&encoded), Ok(cmd));
    }

    #[test]
    fn empty_input_is_truncated() {
        assert_eq!(Command::decode(&[]), Err(DecodeError::Truncated));
    }

    #[test]
    fn unknown_op_is_rejected() {
        assert_eq!(Command::decode(&[9]), Err(DecodeError::UnknownOp(9)));
    }

    #[test]
    fn truncated_value_is_rejected() {
        let mut encoded = Command::Put {
            key: b"key".to_vec(),
            value: b"value".to_vec(),
        }
        .encode();
        encoded.pop();
        assert_eq!(Command::decode(&encoded), Err(DecodeError::Truncated));
    }

    #[test]
    fn trailing_bytes_are_rejected() {
        let mut encoded = Command::Delete { key: b"k".to_vec() }.encode();
        encoded.push(0);
        assert_eq!(Command::decode(&encoded), Err(DecodeError::TrailingBytes));
    }
}
