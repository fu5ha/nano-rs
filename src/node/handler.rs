use nano_lib_rs::message::{MessageBuilder, Message, MessageKind, MessagePayload};

use node::State;
use error::*;
use utils::check_addr;

use std::net::{SocketAddrV6, SocketAddr};
use std::sync::Arc;

use futures::{stream, Stream};

pub fn keepalive(msg: Message, _src: SocketAddrV6, state: Arc<State>)
    -> Box<Stream<Item=(Message, SocketAddr), Error=Error> + Send>
{
    if let MessagePayload::KeepAlive(peer_addrs) = msg.payload {
        let send_peers = state.random_peers(8);
        let msg = MessageBuilder::new(MessageKind::KeepAlive)
            .with_payload(MessagePayload::KeepAlive(send_peers))
            .build();
        let to_send = peer_addrs.into_iter()
            .filter_map(move |peer_addr| {
                if check_addr(peer_addr) {
                    Some((msg.clone(), SocketAddr::V6(peer_addr)))
                } else {
                    None
                }
            });
        let count = state.peer_count();
        debug!("Added peers, new peer count: {}", count);
        Box::new(stream::iter_ok(to_send))
    } else {
        debug!("Malformed Keepalive, no peers added!");
        Box::new(stream::empty())
    }
}

pub fn publish(mut msg: Message, _src: SocketAddrV6, _state: Arc<State>)
    -> Box<Stream<Item=(Message, SocketAddr), Error=Error> + Send>
{
    if let MessagePayload::Publish(ref mut block) =  msg.payload {
        let hash = match block.hash(false) {
            Ok(hash) => hash.into(),
            Err(e) => format!("Error calculating hash for block: {}", e),
        };
        info!("Got {:?} block with hash {}", block.kind, hash);
        Box::new(stream::empty())
    } else {
        debug!("Malformed Publish, ignoring.");
        Box::new(stream::empty())
    }
}

pub fn confirm_req(mut msg: Message, _src: SocketAddrV6, _state: Arc<State>)
    -> Box<Stream<Item=(Message, SocketAddr), Error=Error> + Send>
{

    if let MessagePayload::ConfirmReq(ref mut block) =  msg.payload {
        let hash = match block.hash(false) {
            Ok(hash) => hash.into(),
            Err(e) => format!("Error calculating hash for block: {}", e),
        };
        info!("Got {:?} block with hash {}", block.kind, hash);
        Box::new(stream::empty())
    } else {
        debug!("Malformed ConfirmReq, ignoring.");
        Box::new(stream::empty())
    }
}
 