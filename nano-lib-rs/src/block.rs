extern crate nanopow_rs;
pub use nanopow_rs::{InputHash, Work};

use byteorder::{BigEndian, ByteOrder};

use bytes::{Bytes, BytesMut, BufMut, Buf, IntoBuf};
use blake2::Blake2b;
use blake2::digest::{Input, VariableOutput};

use hash::{Hash, Hasher};
use keys::{SecretKey, PublicKey, Signature, SIGNATURE_LENGTH};
use error::*;

use data_encoding::HEXUPPER;

use std::fmt;

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

    /// View the hash as a byte slice
    pub fn as_bytes<'a>(&'a self) -> &'a [u8; 32] {
        &(self.0)
    }
}

impl Hash for BlockHash {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.0[..])
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

enum_byte!(BlockKind {
    Invalid = 0x00,
    NotABlock = 0x01,
    Send = 0x02,
    Receive = 0x03,
    Open = 0x04,
    Change = 0x05,
    Utx = 0x06,
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
    pub payload: Option<BlockPayload>,
    pub next: Option<BlockHash>,
    pub work: Option<Work>,
    pub signature: Option<Signature>,
    pub hash: Option<BlockHash>
}

impl Block {
    pub fn new(kind: BlockKind, payload: Option<BlockPayload>, signature: Option<Signature>, work: Option<Work>) -> Self {
        Block {
            kind,
            payload,
            next: None,
            work,
            signature,
            hash: None,
        }
    }
    pub fn next(&self) -> Option<BlockHash> {
        self.next
    }
    pub fn signature(&self) -> Option<Signature> {
        self.signature
    }
    pub fn sign(&mut self, _key: &SecretKey) -> Result<()> {
        unimplemented!();
    }
    pub fn work(&self) -> Option<Work> {
        self.work.clone()
    }
    pub fn set_work(&mut self, work: Work) -> Result<()> {
        if let Some(ref p) = self.payload {
            let valid = nanopow_rs::check_work(&p.work_source(), &work);
            if valid {
                bail!(ErrorKind::InvalidWorkError);
            }
            self.work = Some(work);
            Ok(())
        } else {
            bail!("Cannot set work for a block with no payload");
        }
    }
    pub fn generate_work(&mut self) -> Option<Work> {
        if let Some(ref p) = self.payload {
            let work = nanopow_rs::generate_work(&p.work_source(), None);
            self.work = work;
            work
        } else {
            None
        }
    }
    pub fn verify_work(&self) -> Result<bool> {
        if let Some(ref p) = self.payload {
            if let Some(ref w) = self.work {
                return Ok(nanopow_rs::check_work(&p.work_source(), w))
            }
        }
        bail!(ErrorKind::NoWorkError);
    }
    pub fn cached_hash(&self) -> Option<BlockHash> {
        self.hash
    }
    pub fn calculate_hash(&mut self) -> Result<BlockHash> {
        if let Some(ref p) = self.payload {
            let mut hasher = BlockHasher::new();
            p.hash(&mut hasher);
            let hash = hasher.finish()?;
            self.hash = Some(hash);
            Ok(hash)
        } else {
            bail!("Cannot calculate hash for block without payload")
        }
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
        if let Some(ref p) = self.payload {
            let mut buf = BytesMut::new();
            p.serialize_bytes(&mut buf);
            if let Some(ref s) = self.signature {
                buf.reserve(SIGNATURE_LENGTH);
                buf.put_slice(&s.to_bytes());
            }
            if let Some(ref w) = self.work {
                buf.reserve(8);
                buf.put_slice(w.as_ref());
            }
            Bytes::from(buf)
        } else {
            Bytes::with_capacity(0)
        }
    }
    pub fn deserialize_bytes(bytes: Bytes, kind: BlockKind) -> Result<Self> {
        Ok(match kind {
            BlockKind::Invalid | BlockKind::NotABlock => {
                Block::new(kind, None, None, None)
            },
            _ => {
                let len = bytes.len();
                if len < kind.size() + SIGNATURE_LENGTH {
                    bail!(ErrorKind::BlockParseError(BlockParseErrorKind::NoSignature));
                } else if len < kind.size() + SIGNATURE_LENGTH + 8 {
                    bail!(ErrorKind::BlockParseError(BlockParseErrorKind::NoWork));
                }
                let mut buf = bytes.into_buf();
                let payload = BlockPayload::deserialize_bytes(&mut buf, kind)?;
                let mut sig_buf = [0u8; 64];
                buf.copy_to_slice(&mut sig_buf);
                let signature = Signature::from_bytes(&sig_buf)?;
                let mut work_buf = [0u8; 8];
                buf.copy_to_slice(&mut work_buf);
                let work = Work::from_bytes(&work_buf)?;
                Block::new(kind, Some(payload), Some(signature), Some(work))
            }
        })
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

pub trait BufMutExt: BufMut {
    fn put_i128<T: ByteOrder>(&mut self, n: i128) {
        let mut buf = [0u8; 16];
        T::write_i128(&mut buf, n);
        self.put_slice(&buf)
    }

    fn put_u128<T: ByteOrder>(&mut self, n: u128) {
        let mut buf = [0u8; 16];
        T::write_u128(&mut buf, n);
        self.put_slice(&buf)
    }
}

impl<T: BufMut> BufMutExt for T {}

pub trait BufExt: Buf {
    fn get_u128<T: ByteOrder>(&mut self) -> u128 {
        let mut buf = [0; 16];
        self.copy_to_slice(&mut buf);
        T::read_u128(&buf)
    }

    fn get_i128<T: ByteOrder>(&mut self) -> i128 {
        let mut buf = [0; 16];
        self.copy_to_slice(&mut buf);
        T::read_i128(&buf)
    }
}

impl<T: Buf> BufExt for T {}


/// Link field contains source block_hash if receiving, destination account if sending
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Link {
    Source(BlockHash),
    Destination(PublicKey),
    Unknown([u8; 32])
}

// TODO: Process Link properly so that we can remove unkown type
impl Link {
    pub fn as_bytes<'a>(&'a self) -> &'a [u8; 32] {
        match *self {
            Link::Source(ref h) => h.as_bytes(),
            Link::Destination(ref k) => k.as_bytes(),
            Link::Unknown(ref b) => &b
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BlockPayload {
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
        link: Link,
    },
}

impl BlockPayload {
    pub fn work_source(&self) -> InputHash {
        match *self {
            BlockPayload::Send { ref previous, .. } => previous.clone().into(),
            BlockPayload::Receive { ref previous, .. } => previous.clone().into(),
            BlockPayload::Open { ref account, .. } => InputHash::from_bytes(account.clone().to_bytes()).unwrap(),
            BlockPayload::Change { ref previous, .. } => previous.clone().into(),
            BlockPayload::Utx { ref previous, .. } => previous.clone().into(),
        }
    }

    pub fn serialize_bytes(&self, buf: &mut BytesMut) {
        match *self {
            BlockPayload::Send {
                ref previous,
                ref destination,
                ref balance,
            } => {
                buf.reserve(BlockKind::Send.size());
                buf.put_slice(previous.as_bytes());
                buf.put_slice(destination.as_bytes());
                buf.put_u128::<BigEndian>(*balance);
            }
            BlockPayload::Receive {
                ref previous,
                ref source,
            } => {
                buf.reserve(BlockKind::Receive.size());
                buf.put_slice(previous.as_bytes());
                buf.put_slice(source.as_bytes());
            }
            BlockPayload::Open {
                ref source,
                ref representative,
                ref account,
            } => {
                buf.reserve(BlockKind::Open.size());
                buf.put_slice(source.as_bytes());
                buf.put_slice(representative.as_bytes());
                buf.put_slice(account.as_bytes());
            }
            BlockPayload::Change {
                ref previous,
                ref representative,
            } => {
                buf.reserve(BlockKind::Change.size());
                buf.put_slice(previous.as_bytes());
                buf.put_slice(representative.as_bytes());
            }
            BlockPayload::Utx {
                ref account,
                ref previous,
                ref representative,
                ref balance,
                ref link,
            } => {
                buf.reserve(BlockKind::Utx.size());
                buf.put_slice(account.as_bytes());
                buf.put_slice(previous.as_bytes());
                buf.put_slice(representative.as_bytes());
                buf.put_u128::<BigEndian>(*balance);
                buf.put_slice(link.as_bytes());
            }
        }
    }

    pub fn deserialize_bytes<B: BufExt>(buf: &mut B, kind: BlockKind) -> Result<Self> {
        Ok(match kind {
            BlockKind::Send => {
                if buf.remaining() < BlockKind::Send.size() {
                    bail!(ErrorKind::BlockPayloadLengthError(kind, buf.remaining()));
                }
                let mut temp_buf = [0u8; 32];
                buf.copy_to_slice(&mut temp_buf);
                let previous = BlockHash::from_bytes(&temp_buf)?;
                buf.copy_to_slice(&mut temp_buf);
                let destination = PublicKey::from_bytes(&temp_buf)?;
                let balance = buf.get_u128::<BigEndian>();
                BlockPayload::Send { previous, destination, balance }
            }
            BlockKind::Receive => {
                if buf.remaining() < BlockKind::Receive.size() {
                    bail!(ErrorKind::BlockPayloadLengthError(kind, buf.remaining()));
                }
                let mut temp_buf = [0u8; 32];
                buf.copy_to_slice(&mut temp_buf);
                let previous = BlockHash::from_bytes(&temp_buf)?;
                buf.copy_to_slice(&mut temp_buf);
                let source = BlockHash::from_bytes(&temp_buf)?;
                BlockPayload::Receive { previous, source }
            }
            BlockKind::Open => {
                if buf.remaining() < BlockKind::Open.size() {
                    bail!(ErrorKind::BlockPayloadLengthError(kind, buf.remaining()));
                }
                let mut temp_buf = [0u8; 32];
                buf.copy_to_slice(&mut temp_buf);
                let source = BlockHash::from_bytes(&temp_buf)?;
                buf.copy_to_slice(&mut temp_buf);
                let representative = PublicKey::from_bytes(&temp_buf)?;
                buf.copy_to_slice(&mut temp_buf);
                let account = PublicKey::from_bytes(&temp_buf)?;
                BlockPayload::Open { source, representative, account }
            }
            BlockKind::Change => {
                if buf.remaining() < BlockKind::Change.size() {
                    bail!(ErrorKind::BlockPayloadLengthError(kind, buf.remaining()));
                }
                let mut temp_buf = [0u8; 32];
                buf.copy_to_slice(&mut temp_buf);
                let previous = BlockHash::from_bytes(&temp_buf)?;
                buf.copy_to_slice(&mut temp_buf);
                let representative = PublicKey::from_bytes(&temp_buf)?;
                BlockPayload::Change { previous, representative }
            }
            BlockKind::Utx => {
                if buf.remaining() < BlockKind::Utx.size() {
                    bail!(ErrorKind::BlockPayloadLengthError(kind, buf.remaining()));
                }
                let mut temp_buf = [0u8; 32];
                buf.copy_to_slice(&mut temp_buf);
                let account = PublicKey::from_bytes(&temp_buf)?;
                buf.copy_to_slice(&mut temp_buf);
                let previous = BlockHash::from_bytes(&temp_buf)?;
                buf.copy_to_slice(&mut temp_buf);
                let representative = PublicKey::from_bytes(&temp_buf)?;
                let balance = buf.get_u128::<BigEndian>();
                buf.copy_to_slice(&mut temp_buf);
                // TODO: Process link properly
                let link = Link::Unknown(temp_buf);
                BlockPayload::Utx { account, previous, representative, balance, link }
            }
            _ => bail!(ErrorKind::InvalidBlockPayloadKindError(kind))
        })
    }
}

impl Hash for BlockPayload {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match *self {
            BlockPayload::Send {
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
            BlockPayload::Receive {
                ref previous,
                ref source,
            } => {
                previous.hash(state);
                source.hash(state);
            }
            BlockPayload::Open {
                ref source,
                ref representative,
                ref account,
            } => {
                source.hash(state);
                representative.hash(state);
                account.hash(state);
            }
            BlockPayload::Change {
                ref previous,
                ref representative,
            } => {
                previous.hash(state);
                representative.hash(state);
            }
            BlockPayload::Utx {
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
                state.write(link.as_bytes());
            }
        }
    }
}
