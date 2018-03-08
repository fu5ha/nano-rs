#[derive(Clone, Copy)]
#[repr(u8)]
pub enum MessageKind {
  InvalidMessage,
  NotAMessage,
  KeepAliveMessage,
  PublishMessage,
  ConfirmReqMessage,
  ConfirmAckMessage,
  BulkPullMessage,
  BulkPushMessage,
  FrontierReqMessage
}

pub const MAGIC_NUMBER: u8 = 0x52;

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum NetworkKind {
  Test = 'A' as u8,
  Beta = 'B' as u8,
  Main = 'C' as u8,
}

#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Version {
  One = 0x01,
  Two = 0x02,
  Three = 0x03,
  Four = 0x04,
  Five = 0x05,
  Six = 0x06,
}

bitflags! {
  pub struct Extensions: u16 {
    const IPV4_ONLY = 1;
    const BOOTSTRAP_NODE = 2;
  }
}

pub struct MessageHeader {
  pub magic_number: u8,
  pub network: NetworkKind,
  pub version_max: Version,
  pub version_using: Version,
  pub version_min: Version,
  pub kind: MessageKind,
  pub extensions: Extensions,
}
