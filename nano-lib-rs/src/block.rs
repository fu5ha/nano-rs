extern crate nanopow_rs;
use nanopow_rs::{check_work, generate_work, InputHash, Work};

// use bytes::{Bytes};
use blake2::Blake2b;
use blake2::digest::{Input, VariableOutput};

use hash::{Hash, Hasher};
use keys::{PrivateKey, PublicKey};
use error::*;

use data_encoding::HEXUPPER;

enum_byte!(BlockKind {
    Invalid = 0,
    NotABlock = 1,
    SendBlock = 2,
    ReceiveBlock = 3,
    OpenBlock = 4,
    ChangeBlock = 5,
    UniversalBlock = 6, // not implemented
});

impl BlockKind {
    pub fn size(&self) -> usize {
        match *self {
            BlockKind::Invalid => 0,
            BlockKind::NotABlock => 0,
            BlockKind::SendBlock => 80,
            BlockKind::ReceiveBlock => 64,
            BlockKind::OpenBlock => 96,
            BlockKind::ChangeBlock => 32,
            BlockKind::UniversalBlock => 0, // not implemented
        }
    }
}

#[derive(Clone, Copy)]
pub struct BlockHash([u8; 32]);

impl BlockHash {
    /// Convert hexadecimal formatted data into a BlockHash
    pub fn from_hex<T: AsRef<[u8]>>(s: T) -> Result<Self> {
        let bytes = s.as_ref();
        if bytes.len() != 64 {
            bail!(ErrorKind::BlockHashLengthError);
        }
        let mut buf = [0u8; 32];
        let _ = HEXUPPER
            .decode_mut(bytes, &mut buf)
            .map_err::<Error, _>(|e| ErrorKind::InvalidHexCharacterError(e.error.position).into())?;
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
        let string = HEXUPPER.encode(&hash.0);
        string
    }
}

impl From<BlockHash> for InputHash {
    fn from(hash: BlockHash) -> Self {
        InputHash::new(hash.0)
    }
}

#[derive(Clone, Copy)]
pub struct Signature([u8; 32]);

impl Signature {
    /// Convert hexadecimal formatted data into an InputHash
    pub fn from_hex<T: AsRef<[u8]>>(s: T) -> Result<Self> {
        let bytes = s.as_ref();
        if bytes.len() != 64 {
            bail!(ErrorKind::BlockHashLengthError);
        }
        let mut buf = [0u8; 32];
        let _ = HEXUPPER
            .decode_mut(bytes, &mut buf)
            .map_err::<Error, _>(|e| ErrorKind::InvalidHexCharacterError(e.error.position).into())?;
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
        let string = HEXUPPER.encode(&sig.0);
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
                return Ok(hash);
            }
        }
        self.calculate_hash()
    }
}

pub struct BlockHasher {
    blake: Blake2b,
}

impl BlockHasher {
    pub fn new() -> Self {
        BlockHasher {
            blake: Blake2b::new(32).unwrap(),
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
        self.blake
            .variable_result(&mut buf)
            .map_err(|_| "Invalid key length")?;
        Ok(BlockHash(buf))
    }
}

pub struct RawSendBlock {
    previous: BlockHash,
    destination: BlockHash,
    balance: u128,
}

impl Hash for RawSendBlock {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.previous.hash(state);
        self.destination.hash(state);
        self.balance.hash(state);
    }
}

pub struct RawReceiveBlock {
    previous: BlockHash,
    source: BlockHash,
}

impl Hash for RawReceiveBlock {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.previous.hash(state);
        self.source.hash(state);
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

pub struct RawChangeBlock {
    previous: BlockHash,
    representative: BlockHash,
}

impl Hash for RawChangeBlock {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.previous.hash(state);
        self.representative.hash(state);
    }
}

macro_rules! create_block {
    ( $( $block_name:ident: $raw_block_type:ty),+ ) => {$(
        pub struct $block_name {
            inner: $raw_block_type,
            next: Option<BlockHash>,
            work: Option<Work>,
            signature: Option<Signature>,
            hash: Option<BlockHash>
        }
    )*}
}

create_block! {
    SendBlock: RawSendBlock,
    ReceiveBlock: RawReceiveBlock,
    OpenBlock: RawOpenBlock,
    ChangeBlock: RawChangeBlock
}

macro_rules! impl_block {
    ( $( $block_name:ident: $work_source:ident ),+ ) => {$(
        impl Block for $block_name {
            fn kind(&self) -> BlockKind {
                BlockKind::$block_name
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
            fn sign(&mut self, _key: &PrivateKey) -> Result<()> {
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
                let work = generate_work(&self.inner.$work_source.clone().into(), None);
                self.work = work;
            }
            fn check_work(&self, work: &Work) -> bool {
                check_work(&self.inner.$work_source.clone().into(), work)
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
    )*}
}

impl_block! {
    SendBlock: previous,
    ReceiveBlock: previous,
    OpenBlock: account,
    ChangeBlock: previous
}
