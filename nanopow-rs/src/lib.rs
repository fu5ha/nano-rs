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

use blake2::{Blake2b};
use blake2::digest::{Input, VariableOutput};

use data_encoding::{HEXLOWER, HEXUPPER};

use rand::{XorShiftRng, Rng, SeedableRng};

/// Error types, using error-chain
pub mod error;
use error::*;

/// An 8 byte array used to represent the work value
#[derive(Clone)]
pub struct Work([u8; 8]);

impl Work
{
    /// Create Work from properly sized byte array
    pub fn new(bytes: [u8; 8]) -> Self {
        Work(bytes)
    }

    /// Convert hexadecimal formatted data into a Work value
    pub fn from_hex<T: AsRef<[u8]>>(s: T) -> Result<Self> {
        let bytes = s.as_ref();
        if bytes.len() != 16 {
            bail!(ErrorKind::WorkLengthError);
        }
        let mut buf = [0u8; 8];
        let _ = HEXLOWER.decode_mut(bytes, &mut buf)
            .map_err::<Error, _>(|e| ErrorKind::InvalidHexCharacterError(e.error.position).into())?;
        Ok(Work(buf))
    }

    /// Create Work from a raw byte slice
    pub fn from_bytes<T: AsRef<[u8]>>(bytes: T) -> Result<Self> {
        let bytes = bytes.as_ref();
        if bytes.len() != 8 {
            bail!(ErrorKind::WorkLengthError);
        }
        let mut buf = [0u8; 8];
        for i in 0..8 {
            buf[i] = bytes[i];
        }
        Ok(Work(buf))
    }
}

impl AsRef<[u8]> for Work {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<Work> for String {
    fn from(work: Work) -> Self {
        let string = HEXLOWER.encode(&work.0);
        string
    }
}

/// A 32 byte array used to represent a valid input hash
#[derive(Clone)]
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

fn check_result_threshold(hash: &[u8]) -> bool {
    let first = (&hash[5..8]).iter().fold(true, |acc, &byte| {
        acc && byte == 255
    });
    hash[4] >= 192 && first
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
        Some(Work(w))
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
                    result_valid = check_result_threshold(&output[..]);
                    if has_max_iters {
                        iters += 1;
                    }
                    done = donerx.try_recv().unwrap_or(false);
                }
                if done {
                    return;
                }
                if result_valid {
                    work.reverse();
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
    let mut work = work.0;
    work.reverse();
    let mut hasher = Blake2b::new(8).unwrap();
    hasher.process(&work[..]);
    hasher.process(&hash[..]);
    let mut output = [0u8; 8];
    {
        let result = hasher.variable_result(&mut output);
        if result.is_err() {
            return false;
        }
    }
    check_result_threshold(&output[..])
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
    fn creates_work_from_hex_and_bytes() {
        let hex = String::from("4effb6b0cd5625e2");
        let work = Work::from_hex(&hex).unwrap();
        let work2 = Work::from_bytes(HEXLOWER.decode(hex.as_ref()).unwrap()).unwrap();
        assert!(work.0 == work2.0);
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
