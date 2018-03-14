use bytes::{Bytes, BytesMut, BufMut, Buf, IntoBuf, LittleEndian};
use bincode;
use error::*;
use block::{BlockKind, Block, Signature};
use std::net::{SocketAddrV6, Ipv6Addr};
use std::cmp;
use keys::PublicKey;

enum_byte!(MessageKind {
    Invalid = 0x00,
    NotAMessage = 0x01,
    KeepAlive = 0x02,
    Publish = 0x03,
    ConfirmReq = 0x04,
    ConfirmAck = 0x05,
    BulkPull = 0x06,
    BulkPush = 0x07,
    FrontierReq = 0x08,
});

impl MessageKind {
    pub fn size(&self) -> Option<usize> {
        match *self {
            MessageKind::KeepAlive => Some(144),
            _ => None
        }
    }
}

pub const MAGIC_NUMBER: u8 = 0x52;

enum_byte!(NetworkKind {
    Test = 0x41, // 'A' in ASCII
    Beta = 0x42, // 'B' in ASCII
    Main = 0x43, // 'C' in ASCII
});

enum_byte!(Version {
    One = 0x01,
    Two = 0x02,
    Three = 0x03,
    Four = 0x04,
    Five = 0x05,
    Six = 0x06,
});

bitflags! {
  #[derive(Serialize, Deserialize)]
  pub struct Extensions: u8 {
    const IPV4_ONLY = 1;
    const BOOTSTRAP_NODE = 2;
    const NONE = 0;
  }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageHeader {
    pub magic_number: u8,
    pub network: NetworkKind,
    pub version_max: Version,
    pub version_using: Version,
    pub version_min: Version,
    pub kind: MessageKind,
    pub block_kind: BlockKind,
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageInner {
    Invalid,
    KeepAlive(Vec<SocketAddrV6>),
    Publish(Block),
    ConfirmReq(Block),
    ConfirmAck {
        public_key: PublicKey,
        signature: Signature,
        sequence: u64,
        block: Block,
    }
}

impl MessageInner {
    pub fn serialize_bytes(&self) -> Bytes {
        match *self {
            MessageInner::Invalid => {
                Bytes::with_capacity(0)
            },
            MessageInner::KeepAlive(ref peers) => {
                let mut buf = BytesMut::new();
                buf.reserve(MessageKind::KeepAlive.size().unwrap());
                // Official node will only accept exactly 8 peers
                let mut peers = peers.clone();
                for _ in 0..(8 - cmp::min(peers.len(), 8)) {
                    peers.push("[::]:0".parse().unwrap());
                }
                for peer in &peers[..8] {
                    buf.put_slice(&peer.ip().octets()[..]);
                    buf.put_u16::<LittleEndian>(peer.port());
                }
                Bytes::from(buf)
            },
            MessageInner::Publish(ref block) => {
                block.serialize_bytes()
            },
            MessageInner::ConfirmReq(ref block) => {
                block.serialize_bytes()
            },
            MessageInner::ConfirmAck {
                ref public_key,
                ref signature,
                ref sequence,
                ref block,
            } => {
                let mut buf = BytesMut::new();
                buf.reserve(32 + 32 + 8 + block.kind.size());
                buf.put(public_key.as_ref());
                buf.put(signature.as_ref());
                buf.put_u64::<LittleEndian>(*sequence);
                let block_bytes = block.serialize_bytes();
                buf.put(block_bytes);
                Bytes::from(buf)
            },
        }
    }

    pub fn deserialize_bytes(kind: MessageKind, bytes: Bytes) -> Result<Self> {
        Ok(match kind {
            MessageKind::KeepAlive => {
                let peers: Vec<SocketAddrV6> = bytes.chunks(18).filter_map(|chunk| {
                    if chunk.len() == 18 {
                        let mut buf = chunk.into_buf();
                        let mut octets = [0u8; 16];
                        for i in 0..16 {
                            octets[i] = buf.get_u8();
                        }
                        Some(SocketAddrV6::new(Ipv6Addr::from(octets), buf.get_u16::<LittleEndian>(), 0, 0))
                    } else {
                        None
                    }
                }).collect();
                if peers.len() > 0 {
                    MessageInner::KeepAlive(peers)
                } else {
                    MessageInner::Invalid
                }
            },
            _ => {
                MessageInner::Invalid
            }
       })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub header: MessageHeader,
    pub inner: MessageInner,
}

impl Message {
    pub fn new(header: MessageHeader, inner: MessageInner) -> Self {
        Message {
            header,
            inner
        }
    }

    pub fn serialize_bytes(&self) -> Result<Bytes> {
        let header_ser = bincode::serialize(&self.header)?;
        let data = self.inner.serialize_bytes();
        let mut buf = BytesMut::with_capacity(header_ser.len() + data.len());
        buf.put(header_ser);
        buf.put(data);
        Ok(Bytes::from(buf))
    }

    pub fn deserialize_bytes(mut bytes: Bytes) -> Result<Self> {
        let len = bytes.len();
        if bytes.len() < 8 {
            bail!(ErrorKind::MessageHeaderLengthError(len));
        }
        let header_bytes = bytes.split_to(8);
        let header: MessageHeader = bincode::deserialize(&header_bytes)?;
        let inner = MessageInner::deserialize_bytes(header.kind, bytes)?;
        Ok(Message {
            header,
            inner
        })
    }

    pub fn kind(&self) -> MessageKind {
        self.header.kind
    }
}

pub struct MessageBuilder {
    network: Option<NetworkKind>,
    version_max: Option<Version>,
    version_using: Option<Version>,
    version_min: Option<Version>,
    kind: MessageKind,
    block_kind: Option<BlockKind>,
    extensions: Option<Extensions>,
    inner: Option<MessageInner>,
}

impl MessageBuilder {
    pub fn new(kind: MessageKind) -> Self {
        MessageBuilder {
            network: None,
            version_max: None,
            version_using: None,
            version_min: None,
            kind: kind,
            extensions: None,
            block_kind: None,
            inner: None,
        }
    }

    pub fn with_network(mut self, network: NetworkKind) -> Self {
        self.network = Some(network);
        self
    }

    pub fn with_version_max(mut self, version: Version) -> Self {
        self.version_max = Some(version);
        self
    }

    pub fn with_version(mut self, version: Version) -> Self {
        self.version_using = Some(version);
        self
    }

    pub fn with_version_min(mut self, version: Version) -> Self {
        self.version_min = Some(version);
        self
    }

    pub fn with_extensions(mut self, extensions: Extensions) -> Self {
        self.extensions = Some(extensions);
        self
    }

    pub fn with_block_kind(mut self, block_kind: BlockKind) -> Self {
        self.block_kind = Some(block_kind);
        self
    }

    pub fn with_data(mut self, data: MessageInner) -> Self {
        self.inner = Some(data);
        self
    }

    pub fn build(self) -> Message {
        let header = MessageHeader {
            magic_number: MAGIC_NUMBER,
            network: self.network.unwrap_or(NetworkKind::Main),
            version_max: self.version_max.unwrap_or(Version::Five),
            version_using: self.version_using.unwrap_or(Version::Five),
            version_min: self.version_min.unwrap_or(Version::One),
            kind: self.kind,
            block_kind: self.block_kind.unwrap_or(BlockKind::Invalid),
            extensions: self.extensions.unwrap_or(Extensions::NONE),
        };
        let inner = self.inner.unwrap_or(MessageInner::Invalid);
        Message::new(header, inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use data_encoding::HEXUPPER;

    #[test]
    fn deserialize_message() {
        // TODO: Deserialize message body
        let message_raw = Bytes::from(HEXUPPER.decode(b"524305050102000000000000000000000000000000000000A31B00000000000000000000000000000000A31B00000000000000000000000000000000A31B00000000000000000000000000000000A31B00000000000000000000000000000000A31B00000000000000000000000000000000A31B00000000000000000000000000000000A31B00000000000000000000000000000000A31B").unwrap());
        let sock: SocketAddrV6 = "[::]:7075".parse().unwrap();
        let message = Message::deserialize_bytes(message_raw.clone()).expect("should deserialize");
        assert_eq!(message.header.magic_number, MAGIC_NUMBER);
        assert_eq!(message.header.network, NetworkKind::Main);
        assert_eq!(message.header.version_max, Version::Five);
        assert_eq!(message.header.version_using, Version::Five);
        assert_eq!(message.header.version_min, Version::One);
        assert_eq!(message.header.kind, MessageKind::KeepAlive);
        assert_eq!(message.header.block_kind, BlockKind::Invalid);
        assert_eq!(message.header.extensions, Extensions::NONE);
        assert_eq!(message.inner, MessageInner::KeepAlive(vec![sock.clone(); 8]));
    }

    #[test]
    fn serialize_message() {
        let message_raw = Bytes::from(HEXUPPER.decode(b"524305050102000000000000000000000000000000000000A31B00000000000000000000000000000000A31B00000000000000000000000000000000A31B00000000000000000000000000000000A31B00000000000000000000000000000000A31B00000000000000000000000000000000A31B00000000000000000000000000000000A31B00000000000000000000000000000000A31B").unwrap());
        let sock: SocketAddrV6 = "[::]:7075".parse().unwrap();
        let message = MessageBuilder::new(MessageKind::KeepAlive)
            .with_data(MessageInner::KeepAlive(vec![sock.clone(); 8]))
            .build();
        let message_ser = message.serialize_bytes().unwrap();
        assert_eq!(&message_ser[..], &message_raw[..]);
    }
}
