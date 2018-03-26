extern crate nanopow_rs;
pub use nanopow_rs::{InputHash, Work};

use byteorder::{BigEndian, LittleEndian, ByteOrder};

use bytes::{Bytes, BytesMut, BufMut, Buf, IntoBuf};
use blake2::Blake2b;
use blake2::digest::{Input, VariableOutput};

use hash::{Hash, Hasher};
use keys::{SecretKey, PublicKey, Signature, SIGNATURE_LENGTH};
use error::*;

use data_encoding::HEXUPPER;

use std::fmt;

use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer, SerializeStruct};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
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
    State = 0x06,
});

impl BlockKind {
    pub fn payload_size(&self) -> usize {
        match *self {
            BlockKind::Send => 80,
            BlockKind::Receive => 64,
            BlockKind::Open => 96,
            BlockKind::Change => 32,
            BlockKind::State => 144,
            _ => 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    pub payload: BlockPayload,
    pub work: Option<Work>,
    pub signature: Option<Signature>,
    pub next: Option<BlockHash>,
    pub hash: Option<BlockHash>
}

impl Block {
    pub fn new(payload: BlockPayload, signature: Option<Signature>, work: Option<Work>) -> Self {
        Block {
            payload,
            work,
            signature,
            hash: None,
            next: None,
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
        let valid = nanopow_rs::check_work(&self.payload.root(), &work);
        if valid {
            bail!(ErrorKind::InvalidWorkError);
        }
        self.work = Some(work);
        Ok(())
    }
    pub fn generate_work(&mut self) -> Option<Work> {
        let work = nanopow_rs::generate_work(&self.payload.root(), None);
        self.work = work;
        work
    }
    pub fn verify_work(&self) -> Result<bool> {
        if let Some(ref w) = self.work {
            return Ok(nanopow_rs::check_work(&self.payload.root(), w))
        }
        bail!(ErrorKind::NoWorkError);
    }
    pub fn cached_hash(&self) -> Option<BlockHash> {
        self.hash
    }
    pub fn calculate_hash(&mut self) -> Result<BlockHash> {
        let mut hasher = BlockHasher::new();
        self.payload.hash(&mut hasher);
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
        let mut buf = BytesMut::new();
        self.payload.serialize_bytes(&mut buf);
        if let Some(ref s) = self.signature {
            buf.reserve(SIGNATURE_LENGTH);
            buf.put_slice(&s.to_bytes());
        }
        if let Some(ref w) = self.work {
            buf.reserve(8);
            if let BlockPayload::State {..} = self.payload {
                buf.put_u64::<BigEndian>(w.0);
            } else {
                buf.put_u64::<LittleEndian>(w.0);
            }
        }
        Bytes::from(buf)
    }
    pub fn deserialize_bytes(bytes: Bytes, kind: BlockKind) -> Result<Self> {
        Ok(match kind {
            BlockKind::Invalid | BlockKind::NotABlock => {
                bail!("Invalid block kind")
            },
            _ => {
                let len = bytes.len();
                if len < kind.payload_size() + SIGNATURE_LENGTH {
                    bail!(ErrorKind::BlockParseError(BlockParseErrorKind::NoSignature));
                } else if len < kind.payload_size() + SIGNATURE_LENGTH + 8 {
                    bail!(ErrorKind::BlockParseError(BlockParseErrorKind::NoWork));
                }
                let mut buf = bytes.into_buf();
                let payload = BlockPayload::deserialize_bytes(&mut buf, kind)?;
                let mut sig_buf = [0u8; 64];
                buf.copy_to_slice(&mut sig_buf);
                let signature = Signature::from_bytes(&sig_buf)?;
                let work = if kind == BlockKind::State {
                    Work(buf.get_u64::<BigEndian>())
                } else {
                    Work(buf.get_u64::<LittleEndian>())
                };
                Block::new(payload, Some(signature), Some(work))
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
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Link(pub [u8; 32]);

impl Link {
    pub fn as_bytes<'a>(&'a self) -> &'a [u8; 32] {
        &self.0
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum BlockPayload {
    /// Block that sends funds to another Nano account.
    /// Must be `Receive`d by the other account.
    /// 
    /// ### Deprecation Notice
    /// Will soon be deprecated in favor of State blocks.
    Send {
        /// The previous block hash on the account's chain 
        previous: BlockHash,
        /// The destination account's raw ed25519 public key.
        destination: PublicKey,
        /// The balance of the account *after* the send.
        balance: u128,
    },
    /// Block that receives funds from a corresponding
    /// `Send` block.
    /// 
    /// ### Deprecation Notice
    /// Will soon be deprecated in favor of State blocks.
    Receive {
        /// The previous block hash on the account's chain 
        previous: BlockHash,
        /// The block we're receiving.
        source: BlockHash,
    },
    /// The first "receive" in an account chain.
    /// Creates the account, and sets the representative.
    /// 
    /// ### Deprecation Notice
    /// Will soon be deprecated in favor of State blocks.
    Open {
        /// The block we're receiving.
        source: BlockHash,
        /// This account's representative's raw ed25519 public key.
        representative: PublicKey,
        /// The account's raw ed25519 public key.
        account: PublicKey,
    },
    /// Changes the representative for an account.
    /// 
    /// ### Deprecation Notice
    /// Will soon be deprecated in favor of State blocks.
    Change {
        /// The previous block hash on the account's chain 
        previous: BlockHash,
        /// This account's representative's raw ed25519 public key.
        representative: PublicKey,
    },
    /// A block which contains the account's full state.
    State {
        /// This account's raw ed25519 public key.
        account: PublicKey,
        /// The previous block hash on the account's chain. If this is the first
        /// block on a chain, this should be set to all 0s.
        previous: BlockHash,
        /// This account's representative's raw ed25519 public key.
        representative: PublicKey,
        /// The balance of this account *after* this block is processed.
        balance: u128,
        link: Link,
    },
}

impl BlockPayload {
    pub fn root(&self) -> InputHash {
        match *self {
            BlockPayload::Send { ref previous, .. } => previous.clone().into(),
            BlockPayload::Receive { ref previous, .. } => previous.clone().into(),
            BlockPayload::Open { ref account, .. } => InputHash::from_bytes(account.clone().to_bytes()).unwrap(),
            BlockPayload::Change { ref previous, .. } => previous.clone().into(),
            BlockPayload::State { ref previous, .. } => previous.clone().into(),
        }
    }

    pub fn kind(&self) -> BlockKind {
        match *self {
            BlockPayload::Send {..} => BlockKind::Send,
            BlockPayload::Receive {..} => BlockKind::Receive,
            BlockPayload::Open {..} => BlockKind::Open,
            BlockPayload::Change {..} => BlockKind::Change,
            BlockPayload::State {..} => BlockKind::State,
        }
    }

    pub fn size(&self) -> usize {
        self.kind().payload_size()
    }

    pub fn serialize_bytes(&self, buf: &mut BytesMut) {
        match *self {
            BlockPayload::Send {
                ref previous,
                ref destination,
                ref balance,
            } => {
                buf.reserve(self.size());
                buf.put_slice(previous.as_bytes());
                buf.put_slice(destination.as_bytes());
                buf.put_u128::<BigEndian>(*balance);
            }
            BlockPayload::Receive {
                ref previous,
                ref source,
            } => {
                buf.reserve(self.size());
                buf.put_slice(previous.as_bytes());
                buf.put_slice(source.as_bytes());
            }
            BlockPayload::Open {
                ref source,
                ref representative,
                ref account,
            } => {
                buf.reserve(self.size());
                buf.put_slice(source.as_bytes());
                buf.put_slice(representative.as_bytes());
                buf.put_slice(account.as_bytes());
            }
            BlockPayload::Change {
                ref previous,
                ref representative,
            } => {
                buf.reserve(self.size());
                buf.put_slice(previous.as_bytes());
                buf.put_slice(representative.as_bytes());
            }
            BlockPayload::State {
                ref account,
                ref previous,
                ref representative,
                ref balance,
                ref link,
            } => {
                buf.reserve(self.size());
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
                if buf.remaining() < kind.payload_size() {
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
                if buf.remaining() < BlockKind::Receive.payload_size() {
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
                if buf.remaining() < BlockKind::Open.payload_size() {
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
                if buf.remaining() < BlockKind::Change.payload_size() {
                    bail!(ErrorKind::BlockPayloadLengthError(kind, buf.remaining()));
                }
                let mut temp_buf = [0u8; 32];
                buf.copy_to_slice(&mut temp_buf);
                let previous = BlockHash::from_bytes(&temp_buf)?;
                buf.copy_to_slice(&mut temp_buf);
                let representative = PublicKey::from_bytes(&temp_buf)?;
                BlockPayload::Change { previous, representative }
            }
            BlockKind::State => {
                if buf.remaining() < BlockKind::State.payload_size() {
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
                let link = Link(temp_buf);
                BlockPayload::State { account, previous, representative, balance, link }
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
            BlockPayload::State {
                ref account,
                ref previous,
                ref representative,
                ref balance,
                ref link,
            } => {
                state.write(&[0u8; 31]);
                state.write(&[BlockKind::State as u8]); // block type code
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
