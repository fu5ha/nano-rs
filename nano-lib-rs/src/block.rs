extern crate nanopow_rs;
use nanopow_rs::{InputHash, Work};

use byteorder::{BigEndian, ByteOrder};

use bytes::{Bytes, BytesMut, BufMut};
use blake2::Blake2b;
use blake2::digest::{Input, VariableOutput};

use hash::{Hash, Hasher};
use keys::{PrivateKey, PublicKey};
use error::*;

use data_encoding::HEXUPPER;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

enum_byte!(BlockKind {
    Invalid = 0,
    NotABlock = 1,
    Send = 2,
    Receive = 3,
    Open = 4,
    Change = 5,
    Utx = 6,
});

impl BlockKind {
    pub fn size(&self) -> usize {
        match *self {
            BlockKind::Invalid => 0,
            BlockKind::NotABlock => 0,
            BlockKind::Send => 80,
            BlockKind::Receive => 64,
            BlockKind::Open => 96,
            BlockKind::Change => 32,
            BlockKind::Utx => 144,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub kind: BlockKind,
    pub inner: BlockInner,
    pub next: Option<BlockHash>,
    pub work: Option<Work>,
    pub signature: Option<Signature>,
    pub hash: Option<BlockHash>
}

impl Block {
    pub fn next(&self) -> Option<BlockHash> {
        self.next
    }
    pub fn signature(&self) -> Option<Signature> {
        self.signature
    }
    pub fn sign(&mut self, _key: &PrivateKey) -> Result<()> {
        unimplemented!();
    }
    pub fn work(&self) -> Option<Work> {
        self.work.clone()
    }
    pub fn set_work(&mut self, work: Work) -> Result<()> {
        if !self.check_work(&work) {
            bail!(ErrorKind::InvalidWorkError);
        }
        self.work = Some(work);
        Ok(())
    }
    pub fn generate_work(&mut self) {
        let work = nanopow_rs::generate_work(&self.inner.work_source().into(), None);
        self.work = work;
    }
    pub fn check_work(&self, work: &Work) -> bool {
        nanopow_rs::check_work(&self.inner.work_source().into(), work)
    }
    pub fn cached_hash(&self) -> Option<BlockHash> {
        self.hash
    }
    pub fn calculate_hash(&mut self) -> Result<BlockHash> {
        let mut hasher = BlockHasher::new();
        self.inner.hash(&mut hasher);
        let hash = hasher.finish()?;
        self.hash = Some(hash);
        Ok(hash)
    }
    pub fn is_signed(&self) -> bool {
        self.signature().is_some()
    }
    pub fn has_work(&self) -> bool {
        self.work().is_some()
    }
    pub fn hash(&mut self, force: bool) -> Result<BlockHash> {
        if !force {
            let cached_hash = self.cached_hash();
            if let Some(hash) = cached_hash {
                return Ok(hash);
            }
        }
        self.calculate_hash()
    }
    pub fn serialize_bytes(&self) -> Bytes {
        self.inner.serialize_bytes()
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BlockInner {
    Send {
        previous: BlockHash,
        destination: PublicKey,
        /// The balance of the account *after* the send.
        balance: u128,
    },
    Receive {
        previous: BlockHash,
        /// The block we're receiving.
        source: BlockHash,
    },
    /// The first "receive" in an account chain.
    /// Creates the account, and sets the representative.
    Open {
        /// The block we're receiving.
        source: BlockHash,
        representative: PublicKey,
        account: PublicKey,
    },
    /// Changes the representative for an account.
    Change {
        previous: BlockHash,
        representative: PublicKey,
    },
    /// A universal transaction which contains the account state.
    Utx {
        account: PublicKey,
        previous: BlockHash,
        representative: PublicKey,
        balance: u128,
        /// Link field contains source block_hash if receiving, destination account if sending
        link: [u8; 32],
    },
}

impl BlockInner {
    pub fn work_source(&self) -> InputHash {
        match *self {
            BlockInner::Send { ref previous, .. } => previous.clone().into(),
            BlockInner::Receive { ref previous, .. } => previous.clone().into(),
            BlockInner::Open { ref account, .. } => account.clone().into(),
            BlockInner::Change { ref previous, .. } => previous.clone().into(),
            BlockInner::Utx { ref previous, .. } => previous.clone().into(),
        }
    }
    pub fn serialize_bytes(&self) -> Bytes {
        let mut buf = BytesMut::new();
        match *self {
            BlockInner::Send {
                ref previous,
                ref destination,
                ref balance,
            } => {
                buf.reserve(BlockKind::Send.size());
                buf.put(previous.as_ref());
                buf.put(destination.as_ref());
                let mut bal_buf = [0u8; 16];
                BigEndian::write_u128(&mut bal_buf, *balance);
                buf.put(&bal_buf[..]);
            }
            BlockInner::Receive {
                ref previous,
                ref source,
            } => {
                buf.reserve(BlockKind::Receive.size());
                buf.put(previous.as_ref());
                buf.put(source.as_ref());
            }
            BlockInner::Open {
                ref source,
                ref representative,
                ref account,
            } => {
                buf.reserve(BlockKind::Open.size());
                buf.put(source.as_ref());
                buf.put(representative.as_ref());
                buf.put(account.as_ref());
            }
            BlockInner::Change {
                ref previous,
                ref representative,
            } => {
                buf.reserve(BlockKind::Change.size());
                buf.put(previous.as_ref());
                buf.put(representative.as_ref());
            }
            BlockInner::Utx {
                ref account,
                ref previous,
                ref representative,
                ref balance,
                ref link,
            } => {
                buf.reserve(BlockKind::Utx.size());
                let mut block_kind_code = [0u8; 32];
                block_kind_code[31] = BlockKind::Utx as u8;
                buf.put(&block_kind_code[..]);
                buf.put(account.as_ref());
                buf.put(previous.as_ref());
                buf.put(representative.as_ref());
                let mut bal_buf = [0u8; 16];
                BigEndian::write_u128(&mut bal_buf, *balance);
                buf.put(&bal_buf[..]);
                buf.put(&link[..]);
            }
        }
        Bytes::from(buf)
    }
}

impl Hash for BlockInner {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match *self {
            BlockInner::Send {
                ref previous,
                ref destination,
                ref balance,
            } => {
                previous.hash(state);
                destination.hash(state);
                let mut buf = [0u8; 16];
                BigEndian::write_u128(&mut buf, *balance);
                state.write(&buf);
            }
            BlockInner::Receive {
                ref previous,
                ref source,
            } => {
                previous.hash(state);
                source.hash(state);
            }
            BlockInner::Open {
                ref source,
                ref representative,
                ref account,
            } => {
                source.hash(state);
                representative.hash(state);
                account.hash(state);
            }
            BlockInner::Change {
                ref previous,
                ref representative,
            } => {
                previous.hash(state);
                representative.hash(state);
            }
            BlockInner::Utx {
                ref account,
                ref previous,
                ref representative,
                ref balance,
                ref link,
            } => {
                state.write(&[0u8; 31]);
                state.write(&[BlockKind::Utx as u8]); // block type code
                account.hash(state);
                previous.hash(state);
                representative.hash(state);
                let mut buf = [0u8; 16];
                BigEndian::write_u128(&mut buf, *balance);
                state.write(&buf);
                state.write(link);
            }
        }
    }
}
