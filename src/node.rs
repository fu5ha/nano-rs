use net::codec::MessageCodec;
use net::{UdpFramed};

use nano_lib_rs::message::{MessageBuilder, MessageKind, MessageInner, NetworkKind};
use nano_lib_rs;

use tokio::prelude::*;
use tokio::executor::thread_pool::Sender;
use tokio::net::{UdpSocket};
use futures::{self, Future};
use futures::sync::mpsc;

use std::net::{SocketAddr, SocketAddrV6};
use std::sync::{Arc, Mutex};

use tokio_timer::*;
use std::time::{Duration, Instant};

use indexmap::IndexMap;
use indexmap::map::{Entry};
use rand::{self, Rng};

use error::*;

use utils::log_errors;

const KEEPALIVE_INTERVAL: u64 = 60;
#[allow(dead_code)]
const KEEPALIVE_CUTOFF: u64 = KEEPALIVE_INTERVAL * 5;

#[derive(Clone, Copy, Debug)]
struct PeerInfo {
    last_seen: Instant
}

impl Default for PeerInfo {
    fn default() -> Self {
        PeerInfo {
            last_seen: Instant::now()
        }
    }
}

type Peers = IndexMap<SocketAddrV6, PeerInfo>;

struct State {
    pub peers: Mutex<Peers>
}

impl State {
    pub fn new(initial_peers: Peers) -> Self {
        State {
            peers: Mutex::new(initial_peers)
        }
    }

    pub fn peer_count(&self) -> usize {
        self.peers.lock().unwrap().len()
    }

    pub fn add_peer(&self, peer: SocketAddrV6) {
        let mut map = self.peers.lock().unwrap();
        match map.entry(peer) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().last_seen = Instant::now();
            },
            Entry::Vacant(entry) => {
                entry.insert(PeerInfo::default());
            }
        }
    }

    pub fn random_peers(&self, n: usize) -> Vec<SocketAddrV6> {
        let mut rng = rand::thread_rng();
        let peers = self.peers.lock().unwrap();
        (0..n).into_iter().map(|_| {
            let idx = rng.gen_range::<usize>(0, peers.len());
            peers.get_index(idx).unwrap().0.clone()
        }).collect()
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

pub fn run(config: NodeConfig, handle: &Sender) -> Result<impl Future<Item = (), Error = ()>> {
    let socket = UdpSocket::bind(&config.listen_addr)?;

    info!("Listening on: {}", socket.local_addr()?);

    let (sink, stream) = UdpFramed::new(socket, MessageCodec::new()).split();

    let initial_peers: IndexMap<SocketAddrV6, PeerInfo> = config.peers.into_iter()
        .map(|addr| {
            (to_ipv6(addr), PeerInfo::default())
        }).collect();

    let state = Arc::new(State::new(initial_peers));

    let state_handle_process = state.clone();
    
    let process_handler = stream.for_each(move |content| {
            let state = &state_handle_process;
            let kind = content.0.kind();
            state.add_peer(to_ipv6(content.1));
            info!("Received message of kind: {:?} from {}", kind, content.1);
            if let MessageInner::KeepAlive(new_peers) = content.0.inner {
                for new_peer in new_peers {
                    state.add_peer(new_peer);
                }
                let count = state.peer_count();
                info!("Added peers, new peer count: {}", count);
            }
            futures::future::ok(())
        });

    let state_handle_keepalive = state.clone();

    let timer = Timer::default();
    let keepalive_handler = stream::once(Ok(()))
        .chain(timer.interval(Duration::from_secs(KEEPALIVE_INTERVAL)))
        .map(move |_| {
            let count = state.peer_count();
            info!("Sending keepalives to peers. Current peer count: {}", count);
            let state = state_handle_keepalive.clone();
            let peers = state.peers.lock().unwrap();
            let inner_state = state.clone();
            stream::iter_ok::<_, ()>(peers.clone().into_iter()).map(move |(addr, _)| {
                let send_peers = inner_state.random_peers(8);
                let msg = MessageBuilder::new(MessageKind::KeepAlive)
                    .with_data(MessageInner::KeepAlive(send_peers))
                    .build();
                (msg, SocketAddr::V6(addr))
            })
        })
        .flatten()
        .map_err(|e| format!("Got error: {:?}", e).into());

    let (sock_send, sock_recv) = mpsc::channel::<(nano_lib_rs::message::Message, SocketAddr)>(2048);
    // let process_send = sock_send.clone();
    let keepalive_send = sock_send.clone();

    handle.spawn(
        process_handler
            .map_err(|e| error!("Fatal error processing keepalives: {:?}", e))
    ).expect("Could not spawn tokio process");

    handle.spawn(
        keepalive_send
            .sink_map_err(|e| error!("Fatal sending keepalive: {:?}", e))
            .send_all(
                log_errors(keepalive_handler)
                    .map_err(|e| error!("Fatal error processing keepalives: {:?}", e)))
                .map(|_| ())
    ).expect("Could not spawn tokio process");

    Ok(sink
        .sink_map_err(|e| error!("Fatal error sending message: {:?}", e))
        .send_all(sock_recv)
        .map(|_| ())
        .map_err(|e| error!("Got error: {:?}", e)))
}