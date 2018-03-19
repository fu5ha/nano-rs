use bytes::{Bytes, BytesMut, BufMut};
use nano_lib_rs::message::{Message, MessageKind, MessageBuilder};
use tokio_io::codec::{Decoder, Encoder};
use error::*;

pub struct MessageCodec(());

impl MessageCodec {
    pub fn new() -> Self {
        MessageCodec(())
    }
}

impl Decoder for MessageCodec {
    type Item = Message;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>> {
        trace!("Deserializing message: {:?}", &buf[..]);
        let bytes = Bytes::from(buf.take());
        let message = match Message::deserialize_bytes(bytes) {
            Ok(m) => m,
            Err(e) => {
                error!("Error deserializing message: {}", e);
                MessageBuilder::new(MessageKind::Invalid).build()
            }
        };
        Ok(Some(message))
    }
}

impl Encoder for MessageCodec {
    type Item = Message;
    type Error = Error;

    fn encode(&mut self, msg: Message, dst: &mut BytesMut) -> Result<()> {
        let msg_ser = msg.serialize_bytes()?;
        trace!("Serialized message: {:?}", &msg_ser[..]);
        dst.reserve(msg_ser.len());
        dst.put(msg_ser);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::{BytesMut};
    use data_encoding::{HEXUPPER};
    use std::net::SocketAddrV6;
    use nano_lib_rs::message::{MessagePayload};
    use nano_lib_rs::block::{Block, BlockPayload, BlockKind, BlockHash};

    #[test]
    fn encode_decode() {
        let addr: SocketAddrV6 = "[::]:7075".parse().unwrap();
        let message = MessageBuilder::new(MessageKind::KeepAlive)
            .with_payload(MessagePayload::KeepAlive(vec![addr.clone(); 8]))
            .build();
        let mut buf = BytesMut::new();
        let mut a_codec = MessageCodec::new();

        a_codec.encode(message.clone(), &mut buf).expect("should encode keepalive");
        let res = a_codec.decode(&mut buf).unwrap().expect("should decode keepalive");
        assert_eq!(message, res);

        let dummy_data = [0u8; 32];
        let block = Block::new(
            BlockKind::Receive,
            Some(BlockPayload::Receive {
                previous: BlockHash::from_bytes(dummy_data).unwrap(),
                source: BlockHash::from_bytes(dummy_data).unwrap(),
            }));
        let message = MessageBuilder::new(MessageKind::Publish)
            .with_block_kind(BlockKind::Receive)
            .with_payload(MessagePayload::Publish(block))
            .build();
        
        a_codec.encode(message.clone(), &mut buf).expect("should encode publish");
        let res = a_codec.decode(&mut buf).unwrap().expect("should decode publish");
        assert_eq!(message, res);
    }

    #[test]
    fn decode_invalid_header() {
        let mut buf = BytesMut::new();
        buf.extend_from_slice(b"\x52");
        let mut codec = MessageCodec::new();
        
        let res = codec.decode(&mut buf).unwrap().expect("should decode");
        assert_eq!(res.kind(), MessageKind::Invalid);
    }

    #[test]
    fn decode_invalid_message_body() {
        let mut buf = BytesMut::from(HEXUPPER.decode(b"5243050501020000").unwrap());
        buf.extend_from_slice(b"\x52");
        let mut codec = MessageCodec::new();
        
        let res = codec.decode(&mut buf).unwrap().expect("should decode");
        assert_eq!(res.kind(), MessageKind::KeepAlive);
        assert_eq!(res.payload, MessagePayload::Invalid);
    }
}
