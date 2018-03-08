use bytes::{Bytes, BytesMut, BufMut};
use bincode;
use super::error::*;

enum_byte!(MessageKind {
    InvalidMessage = 0x00,
    NotAMessage = 0x01,
    KeepAliveMessage = 0x02,
    PublishMessage = 0x03,
    ConfirmReqMessage = 0x04,
    ConfirmAckMessage = 0x05,
    BulkPullMessage = 0x06,
    BulkPushMessage = 0x07,
    FrontierReqMessage = 0x08,
});

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
  pub struct Extensions: u16 {
    const IPV4_ONLY = 1;
    const BOOTSTRAP_NODE = 2;
    const NONE = 0;
  }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageHeader {
    pub magic_number: u8,
    pub network: NetworkKind,
    pub version_max: Version,
    pub version_using: Version,
    pub version_min: Version,
    pub kind: MessageKind,
    pub extensions: Extensions,
}

#[derive(Debug, Serialize)]
pub struct Message {
    pub header: MessageHeader,
    pub data: Bytes,
}

impl Message {
    pub fn from_bytes(mut bytes: Bytes) -> Result<Self> {
        let header_bytes = bytes.split_to(8);
        let header: MessageHeader = bincode::deserialize(&header_bytes[..])?;
        Ok(Message {
            header,
            data: bytes,
        })
    }

    pub fn serialize(&self) -> Result<Bytes> {
        let header_ser = bincode::serialize(&self.header)?;
        let mut buf = BytesMut::with_capacity(header_ser.len() + self.data.len());
        buf.put(header_ser);
        buf.put(self.data.clone());
        Ok(Bytes::from(buf))
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
    extensions: Option<Extensions>,
    data: Option<Bytes>,
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
            data: None,
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

    pub fn with_data(mut self, data: Bytes) -> Self {
        self.data = Some(data);
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
            extensions: self.extensions.unwrap_or(Extensions::NONE),
        };
        Message {
            header,
            data: self.data.unwrap_or(Bytes::with_capacity(0)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use data_encoding::HEXUPPER;

    #[test]
    fn properly_deserializes_message() {
        let message_raw = Bytes::from(HEXUPPER.decode(b"5243050501020000FF").unwrap());
        let message = Message::from_bytes(message_raw.clone()).unwrap();
        assert_eq!(message.header.magic_number, MAGIC_NUMBER);
        assert_eq!(message.header.network, NetworkKind::Main);
        assert_eq!(message.header.version_max, Version::Five);
        assert_eq!(message.header.version_using, Version::Five);
        assert_eq!(message.header.version_min, Version::One);
        assert_eq!(message.kind(), MessageKind::KeepAliveMessage);
        assert_eq!(message.header.extensions, Extensions::NONE);
        assert_eq!(&message.data[..], &message_raw[8..]);
    }

    #[test]
    fn properly_serializes_message() {
        let message_raw = Bytes::from(HEXUPPER.decode(b"5243050501020000FF").unwrap());
        let message = MessageBuilder::new(MessageKind::KeepAliveMessage)
            .with_data(message_raw.slice_from(8))
            .build();
        let message_ser = message.serialize().unwrap();
        assert_eq!(&message_ser[..], &message_raw[..]);
    }
}
