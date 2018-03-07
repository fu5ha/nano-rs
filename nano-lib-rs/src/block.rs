extern crate nanopow_rs;
use::nanopow_rs::{Work, generate_work, check_work};

// use bytes::{Bytes};
use blake2::Blake2b;
use blake2::digest::{Input, VariableOutput};

use super::hash::{Hash, Hasher};
use super::keys::{PublicKey, PrivateKey};
use super::error::*;

use hex::{FromHex, ToHex};

#[derive(Clone, Copy)]
pub enum BlockKind {
    Invalid = 0x00,
    NotABlock = 0x01,
    Send = 0x02,
    Receive = 0x03,
    Open = 0x04,
    Change = 0x05,
    Universal = 0x06 // not implemented
}

impl BlockKind {
    pub fn size(&self) -> usize {
        match *self {
            BlockKind::Invalid => 0,
            BlockKind::NotABlock => 0,
            BlockKind::Send => 80,
            BlockKind::Receive => 64,
            BlockKind::Open => 96,
            BlockKind::Change => 32,
            BlockKind::Universal => 0 // not implemented
        }
    }
}

#[derive(Clone, Copy)]
pub struct BlockHash([u8; 32]);

impl BlockHash
{
    /// Convert hexadecimal formatted data into a BlockHash
    pub fn from_hex<T: AsRef<[u8]>>(s: T) -> Result<Self> {
        if s.as_ref().len() / 2 != 32 {
            bail!(ErrorKind::BlockHashLengthError);
        }
        let bytes = Vec::from_hex(s)?;
        if bytes.len() != 32 {
            bail!(ErrorKind::BlockHashLengthError);
        }
        let mut buf = [0u8; 32];
        for i in 0..32 {
            buf[i] = bytes[i];
        }
        Ok(BlockHash(buf))
    }

    /// Create a BlockHash from a raw byte slice
    pub fn from_bytes<T: AsRef<[u8]>>(bytes: T) -> Result<Self> {
        let bytes = bytes.as_ref();
        if bytes.len() != 32 {
            bail!(ErrorKind::BlockHashLengthError);
        }
        let mut buf = [0u8; 32];
        for i in 0..32 {
            buf[i] = bytes[i];
        }
        Ok(BlockHash(buf))
    }
}

impl Hash for BlockHash {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.0[..])
    }
}

impl AsRef<[u8]> for BlockHash {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<BlockHash> for String {
    fn from(hash: BlockHash) -> Self {
        let mut string = String::with_capacity(64);
        hash.write_hex_upper(&mut string).unwrap();
        string
    }
}

#[derive(Clone, Copy)]
pub struct Signature([u8; 32]);

impl Signature
{
    /// Convert hexadecimal formatted data into an InputHash
    pub fn from_hex<T: AsRef<[u8]>>(s: T) -> Result<Self> {
        if s.as_ref().len() / 2 != 32 {
            bail!(ErrorKind::SignatureLengthError);
        }
        let bytes = Vec::from_hex(s)?;
        if bytes.len() != 32 {
            bail!(ErrorKind::SignatureLengthError);
        }
        let mut buf = [0u8; 32];
        for i in 0..32 {
            buf[i] = bytes[i];
        }
        Ok(Signature(buf))
    }

    /// Create an InputHash from a raw byte slice
    pub fn from_bytes<T: AsRef<[u8]>>(bytes: T) -> Result<Self> {
        let bytes = bytes.as_ref();
        if bytes.len() != 32 {
            bail!(ErrorKind::SignatureLengthError);
        }
        let mut buf = [0u8; 32];
        for i in 0..32 {
            buf[i] = bytes[i];
        }
        Ok(Signature(buf))
    }
}

impl AsRef<[u8]> for Signature {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<Signature> for String {
    fn from(sig: Signature) -> Self {
        let mut string = String::with_capacity(64);
        sig.write_hex_upper(&mut string).unwrap();
        string
    }
}

pub trait Block {
    fn kind(&self) -> BlockKind;
    fn previous(&self) -> Option<BlockHash>;
    fn next(&self) -> Option<BlockHash>;
    fn signature(&self) -> Option<Signature>;
    fn is_signed(&self) -> bool {
        self.signature().is_some()
    }
    fn sign(&mut self, key: &PrivateKey) -> Result<()>;
    fn work(&self) -> Option<Work>;
    fn set_work(&mut self, work: Work) -> Result<()>;
    fn generate_work(&mut self);
    fn check_work(&self, work: &Work) -> bool;
    fn has_work(&self) -> bool {
        self.work().is_some()
    }
    fn cached_hash(&self) -> Option<BlockHash>;
    fn calculate_hash(&mut self) -> Result<BlockHash>;
    fn hash(&mut self, force: bool) -> Result<BlockHash> {
        if !force {
            let cached_hash = self.cached_hash();
            if let Some(hash) = cached_hash {
                return Ok(hash)
            }
        }
        self.calculate_hash()
    }
}

pub struct BlockHasher {
    blake: Blake2b
}

impl BlockHasher {
    pub fn new() -> Self {
        BlockHasher {
            blake: Blake2b::new(32).unwrap()
        }
    }
}

impl Hasher for BlockHasher {
    type Output = BlockHash;
    fn write(&mut self, bytes: &[u8]) {
        self.blake.process(bytes);
    }

    fn finish(self) -> Result<BlockHash> {
        let mut buf = [0u8; 32];
        self.blake.variable_result(&mut buf).map_err(|_| "Invalid key length")?;
        Ok(BlockHash(buf))
    }
}

pub struct RawOpenBlock {
    source: BlockHash,
    representative: PublicKey,
    account: PublicKey,
}

impl Hash for RawOpenBlock {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.source.hash(state);
        self.representative.hash(state);
        self.account.hash(state);
    }
}

pub struct OpenBlock {
    inner: RawOpenBlock,
    next: Option<BlockHash>,
    work: Option<Work>,
    signature: Option<Signature>,
    hash: Option<BlockHash>
}

impl Block for OpenBlock {
    fn kind(&self) -> BlockKind {
        BlockKind::Open
    }
    fn previous(&self) -> Option<BlockHash> {
        None
    }
    fn next(&self) -> Option<BlockHash> {
        self.next
    }
    fn signature(&self) -> Option<Signature> {
        self.signature
    }
    fn sign(&mut self, key: &PrivateKey) -> Result<()> {
        unimplemented!();
    }
    fn work(&self) -> Option<Work> {
        self.work.clone()
    }
    fn set_work(&mut self, work: Work) -> Result<()> {
        if !self.check_work(&work) {
            bail!(ErrorKind::InvalidWorkError);
        }
        self.work = Some(work);
        Ok(())
    }
    fn generate_work(&mut self) {
        let work = generate_work(&self.inner.account.clone().into(), None);
        self.work = work;
    }
    fn check_work(&self, work: &Work) -> bool {
        check_work(&self.inner.account.clone().into(), work)
    }
    fn cached_hash(&self) -> Option<BlockHash> {
        self.hash
    }
    fn calculate_hash(&mut self) -> Result<BlockHash> {
        let mut hasher = BlockHasher::new();
        self.inner.hash(&mut hasher);
        let hash = hasher.finish()?;
        self.hash = Some(hash);
        Ok(hash)
    }
}
