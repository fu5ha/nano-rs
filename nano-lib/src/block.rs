use bytes::{Bytes};
use blake2::{Blake2b, Digest};

use super::hash::{Hash, Hasher}
use super::keys::{PublicKey, PrivateKey, Account};
use super::error::*;

pub enum BlockType {
    Invalid,
    NotABlock,
    Send,
    Receive,
    Open,
    Change,
    Universal
}

pub type BlockHash([u8; 32]);
pub type Work([u8; 8]);

pub trait Block {
    pub fn type(&self) -> BlockType;
    pub fn prevoius(&self) -> BlockHash;
    pub fn serialize(&self) -> Bytes;
    pub fn is_signed(&self) -> bool;
    pub fn has_work(&self) -> 
}

pub struct BlockHasher {
    blake: u64
}

impl BlockHasher {
    pub fn new() -> Self {
        blake: Blake2b::new(32)
    }
}

impl Hasher for BlockHasher {
    type Output = BlockHash;
    fn write(&mut self, bytes: &[u8]) {
        self.blake.input(bytes);
    }

    fn finish(&self) -> Output {
        let mut buf = [u8; 32];
        self.blake.variable_result(&mut buf);
        BlockHash(buf)
    }
}

pub struct RawOpenBlock {
    source: BlockHash,
    representative: PublicKey,
    account: PublicKey,
}

impl RawOpenBlock {
    pub fn from_bytes(bytes: Bytes) -> Result<Self>  {
        Ok(OpenBlock {
            source: BlockHash([0u8; 32]),
            representative: PublicKey([0u8; 32]),
            account: PublicKey([0u8; 32]),
        })
    }
}

impl Hash for RawOpenBlock {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.source.hash(state);
        self.representative.hash(state);
        self.account.hash(state);
    }
}

pub struct OpenBlock {
    raw: RawOpenBlock,
    work: Option<Work>,
    signature: Option<Signature>,
}

impl Block for OpenBlock {
    fn type(&self) -> BlockType {
        BlockType::Open
    }
}
