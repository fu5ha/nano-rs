//! Small Rust library to generate proof of work for the Nano cryptocurrency.
//! Fully parallelized on the CPU. The goal is for this to be easily includable
//! in web applications as WASM using Parcel and stdweb. Currently, it works best
//! as a native POW generation tool used in Rust and Node apps through
//! [nanopow-rs-node](https://github.com/termhn/nanopow-rs-node)
#![recursion_limit = "1024"]
#![deny(missing_docs)]

#[macro_use]
extern crate error_chain;
extern crate blake2;
extern crate rand;
extern crate data_encoding;
extern crate crossbeam_utils;
extern crate crossbeam_channel;
extern crate num_cpus;
extern crate byteorder;
#[macro_use]
extern crate lazy_static;

use blake2::{Blake2b};
use blake2::digest::{Input, VariableOutput};

use data_encoding::{HEXLOWER, HEXUPPER};

use rand::{XorShiftRng, Rng, SeedableRng};

use byteorder::{ByteOrder, LittleEndian, BigEndian};

use std::fmt;

/// Error types, using error-chain
pub mod error;
use error::*;

const THRESHOLD_STR: &[u8] = b"ffffffc000000000";

lazy_static! {
    /// The network threshold
    pub static ref THRESHOLD: [u8; 8] = {
        let mut buf = [0u8; 8];
        let _ = HEXLOWER.decode_mut(THRESHOLD_STR, &mut buf).unwrap();
        buf
    };
}

/// An 8 byte array used to represent the work value
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Work(pub u64);

impl Work
{
    /// Convert hexadecimal formatted data into a Work value.
    /// Hex value is expected to be encoded in *big-endian* byte order.
    pub fn from_hex<T: AsRef<[u8]>>(s: T) -> Result<Self> {
        let bytes = s.as_ref();
        if bytes.len() != 16 {
            bail!(ErrorKind::WorkLengthError);
        }
        let mut buf = [0u8; 8];
        let _ = HEXLOWER.decode_mut(bytes, &mut buf)
            .map_err::<Error, _>(|e| ErrorKind::InvalidHexCharacterError(e.error.position).into())?;
        let work = BigEndian::read_u64(&buf);
        Ok(Work(work))
    }
}

impl From<Work> for String {
    fn from(work: Work) -> Self {
        let mut buf = [0u8; 8];
        BigEndian::write_u64(&mut buf, work.0);
        let string = HEXLOWER.encode(&buf);
        string
    }
}

impl fmt::Display for Work {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let string: String = (*self).into();
        write!(f, "{}", string)
    }
}


/// A 32 byte array used to represent a valid input hash
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputHash([u8; 32]);

impl InputHash
{
    /// Create Work from properly sized byte array
    pub fn new(bytes: [u8; 32]) -> Self {
        InputHash(bytes)
    }

    /// Convert hexadecimal formatted data into an InputHash
    pub fn from_hex<T: AsRef<[u8]>>(s: T) -> Result<Self> {
        let bytes = s.as_ref();
        if bytes.len() != 64 {
            bail!(ErrorKind::WorkLengthError);
        }
        let mut buf = [0u8; 32];
        let _ = HEXUPPER.decode_mut(bytes, &mut buf)
            .map_err::<Error, _>(|e| ErrorKind::InvalidHexCharacterError(e.error.position).into())?;
        Ok(InputHash(buf))
    }

    /// Create an InputHash from a raw byte slice
    pub fn from_bytes<T: AsRef<[u8]>>(bytes: T) -> Result<Self> {
        let bytes = bytes.as_ref();
        if bytes.len() != 32 {
            bail!(ErrorKind::HashLengthError);
        }
        let mut buf = [0u8; 32];
        for i in 0..32 {
            buf[i] = bytes[i];
        }
        Ok(InputHash(buf))
    }

    /// View the hash as a byte slice
    pub fn as_bytes<'a>(&'a self) -> &'a [u8; 32] {
        &(self.0)
    }
}

impl AsRef<[u8]> for InputHash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<InputHash> for String {
    fn from(hash: InputHash) -> Self {
        let string = HEXUPPER.encode(&hash.0);
        string
    }
}

impl fmt::Display for InputHash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let string: String = (*self).into();
        write!(f, "{}", string)
    }
}

fn check_result_threshold(hash: &[u8; 8]) -> bool {
    (&hash).iter().rev().enumerate().fold(true, |acc, (i, &byte)| {
        acc && byte >= THRESHOLD[i]
    })
}

fn hash_work_internal(work: &[u8], hash: &[u8]) -> [u8; 8] {
    let mut hasher = Blake2b::new(8).unwrap();
    hasher.process(&work[..]);
    hasher.process(&hash[..]);
    let mut output = [0u8; 8];
    hasher.variable_result(&mut output).unwrap();
    output
}

/// Attempts to generate valid work for a given `InputHash` (usually a block hash or public key)
/// with optional maximum iterations
pub fn generate_work(hash: &InputHash, max_iters: Option<u64>) -> Option<Work> {
    let hash = hash.0;
    if let Some(w) = generate_work_internal(&hash[..], max_iters) {
        let work = LittleEndian::read_u64(&w);
        Some(Work(work))
    } else {
        None
    }
}

fn generate_work_internal(hash: &[u8], max_iters: Option<u64>) -> Option<[u8; 8]> {
    let numcpus = num_cpus::get();
    let (tx,rx) = crossbeam_channel::bounded::<Option<[u8; 8]>>(numcpus);
    let (donetx, donerx) = crossbeam_channel::bounded::<bool>(numcpus);
    let has_max_iters = max_iters.is_some();
    let max_iters = max_iters.unwrap_or(numcpus as u64 * 2);
    crossbeam_utils::scoped::scope(|scope| {
        for _ in 0..numcpus {
            scope.spawn(|| {
                let mut rng: XorShiftRng = SeedableRng::from_seed(rand::random::<[u32; 4]>());
                let mut work = [0u8; 8];
                let mut iters = 0u64;
                let mut result_valid = false;
                let mut done = donerx.try_recv().unwrap_or(false);
                while !result_valid && !done && iters < max_iters/numcpus as u64 {
                    work = rng.gen::<[u8; 8]>();
                    let output = hash_work_internal(&work[..], hash);
                    result_valid = check_result_threshold(&output);
                    if has_max_iters {
                        iters += 1;
                    }
                    done = donerx.try_recv().unwrap_or(false);
                }
                if done {
                    return;
                }
                if result_valid {
                    let _ = tx.send(Some(work)).is_ok();
                } else {
                    let _ = tx.send(None).is_ok();
                }
            });
        }
        let mut res = rx.recv().unwrap();
        let mut msgs_resvd = 0;
        while res.is_none() && msgs_resvd < numcpus-1 {
            res = rx.recv().unwrap();
            msgs_resvd += 1;
        }
        for _ in 0..numcpus {
            donetx.send(true).unwrap();
        }
        res
    })
}

/// Checks if a given `Work` value is valid for a given `InputHash` (usually a block hash or public key)
pub fn check_work(hash: &InputHash, work: &Work) -> bool {
    let hash = hash.0;
    let mut work_bytes = [0u8; 8];
    LittleEndian::write_u64(&mut work_bytes, work.0);
    let value = hash_work_internal(&work_bytes, &hash);
    check_result_threshold(&value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_input_hash_from_hex_and_bytes() {
        let hex = String::from("8D3E5F07BFF7B7484CDCB392F47009F62997253D28BD98B94BCED95F03C4DA09");
        let hash = InputHash::from_hex(&hex).unwrap();
        let hash2 = InputHash::from_bytes(HEXUPPER.decode(hex.as_ref()).unwrap()).unwrap();
        assert!(hash.0 == hash2.0);
    }

    #[test]
    fn creates_hex_string_from_input_hash() {
        let hex = String::from("8D3E5F07BFF7B7484CDCB392F47009F62997253D28BD98B94BCED95F03C4DA09");
        let hash = InputHash::from_hex(&hex).unwrap();
        let hash_conv: String = hash.into();
        assert!(hex == hash_conv);
    }
    
    #[test]
    fn creates_work_from_hex() {
        let hex = String::from("4effb6b0cd5625e2");
        let work = Work::from_hex(&hex).unwrap();
        assert!(work.0 == 5692469324495070690);
    }

    #[test]
    fn creates_hex_string_from_work() {
        let hex = String::from("4effb6b0cd5625e2");
        let work = Work::from_hex(&hex).unwrap();
        let work_conv: String = work.into();
        assert!(hex == work_conv);
    }

    #[test]
    fn validates_good_work() {
        let hash = InputHash::from_hex("8D3E5F07BFF7B7484CDCB392F47009F62997253D28BD98B94BCED95F03C4DA09").unwrap();
        let work = Work::from_hex("4effb6b0cd5625e2").unwrap();
        let valid = check_work(&hash, &work);
        assert!(valid);
    }

    #[test]
    fn does_not_validate_bad_work() {
        let hash = InputHash::from_hex("8D3E5F07BFF7B7484CDCB392F47009F62997253D28BD98B94BCED95F03C4DA09").unwrap();
        let work = Work::from_hex("4effc680cd5625e2").unwrap();
        let valid = check_work(&hash, &work);
        assert!(valid == false);
    }

    #[test]
    fn generates_valid_work() {
        let hash = InputHash::from_hex("47F694A96653EB497709490776E492EFBB88EBC5C4E95CC0B2C9DCAB1930C36B").unwrap();
        let work = generate_work(&hash, None).unwrap();
        let work_str: String = work.clone().into();
        println!("generated work: {}", work_str);
        let valid = check_work(&hash, &work);
        assert!(valid);
    }
}
