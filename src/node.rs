use net::codec::MessageCodec;
use net::{UdpFramed};

use nano_lib_rs::message::{MessageBuilder, MessageKind, MessageInner, NetworkKind};
use nano_lib_rs;

use tokio::prelude::*;
use tokio;
use tokio::net::{UdpSocket};
use futures::{self, Future};

use std::net::{SocketAddr, SocketAddrV6};
use std::sync::{Arc, Mutex};

use error::*;

struct State {
    pub peers: Mutex<Vec<SocketAddrV6>>
}

impl State {
    pub fn new() -> Self {
        State {
            peers: Mutex::new(Vec::new())
        }
    }

    pub fn add_peer(&self, peer: SocketAddrV6) {
        let mut peers = self.peers.lock().unwrap();
        let is_present = peers.iter().any(|&p| p == peer);
        if !is_present {
            peers.push(peer);
        }
    }

    pub fn peer_count(&self) -> usize {
        self.peers.lock().unwrap().len()
    }
}

fn to_ipv6(addr: SocketAddr) -> SocketAddrV6 {
    match addr {
        SocketAddr::V4(addr) => SocketAddrV6::new(addr.ip().to_ipv6_mapped(), addr.port(), 0, 0),
        SocketAddr::V6(addr) => addr,
    }
}

pub struct NodeConfig {
    pub peers: Vec<SocketAddr>,
    pub listen_addr: SocketAddr,
    pub network: NetworkKind,
}

pub fn run(config: NodeConfig) -> impl Future<Item = (), Error = Error> {
    let socket = UdpSocket::bind(&config.listen_addr)?;

    info!("Listening on: {}", socket.local_addr()?);

    let (sink, stream) = UdpFramed::new(socket, MessageCodec::new()).split();

    let initial_peers: Vec<SocketAddrV6> = config.peers.into_iter()
        .map(|addr| {
            to_ipv6(addr)
        }).collect();

    if let None = initial_peers.get(0) {
        return Err("Could not connect to initial peer".into());
    }

    let state = Arc::new(State::new());
    
    tokio::spawn(
        stream.for_each(move |content| {
            let state = state.clone();
            let kind = content.0.kind();
            state.add_peer(to_ipv6(content.1));
            if let MessageInner::KeepAlive(peers) = content.0.inner {
                for peer in peers {
                    state.add_peer(peer);
                }
            }
            let count = state.peer_count();
            info!("Received message of kind: {:?} from {}; Current peer count: {}", kind, content.1, count);
            futures::future::ok(())
        })
        .map(|_| ())
        .map_err(|e| error!("Error handling stream: {:?}", e))
    );

    let init_msgs = stream::iter_ok::<_, nano_lib_rs::error::Error>(initial_peers.clone().into_iter()).map(move |peer| {
        info!("Sending keepalive to initial peer: {}", peer);
        let msg = MessageBuilder::new(MessageKind::KeepAlive)
            .with_data(MessageInner::KeepAlive(initial_peers.clone()))
            .build();
        (msg, SocketAddr::V6(peer))
    });

    sink.send_all(init_msgs)
        .map(|_| ())
}