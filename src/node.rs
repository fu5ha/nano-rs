use net::codec::MessageCodec;
use net::{UdpFramed};

use nano_lib_rs::message::{MessageBuilder, Message, MessageKind, MessagePayload, NetworkKind};
use nano_lib_rs;

use tokio;
use tokio::prelude::*;
use tokio::net::{UdpSocket};
use futures::{self, Future};
use futures::sync::mpsc;

use std::net::{SocketAddr, SocketAddrV6};
use net2::UdpBuilder;
use std::sync::{Arc, RwLock};

use tokio_timer::{Timer, TimerError};
use std::time::{Duration, Instant};

use indexmap::IndexMap;
use indexmap::map::{Entry};
use rand::{self, Rng};

use error::*;

use utils::{log_errors, check_addr, to_ipv6};

const KEEPALIVE_INTERVAL: u64 = 60;
const KEEPALIVE_CUTOFF: u64 = KEEPALIVE_INTERVAL * 5;

const PEER_PRUNE_INTERVAL: u64 = KEEPALIVE_INTERVAL * 2;

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
    peers: RwLock<Peers>,
    inactive_peers: RwLock<Peers>,
}

impl State {
    pub fn new(initial_peers: Peers) -> Self {
        State {
            peers: RwLock::new(initial_peers),
            inactive_peers: RwLock::new(IndexMap::new()),
        }
    }

    pub fn peer_count(&self) -> usize {
        self.peers.read().unwrap().len()
    }

    pub fn add_or_update_peer(&self, peer: SocketAddrV6, force: bool) -> bool {
        if !force {
            let inactive_map = self.inactive_peers.read().unwrap();
            if let Some(_) = inactive_map.get(&peer) {
                return false;
            }
        }
        let mut inactive_map = self.inactive_peers.write().unwrap();
        let mut map = self.peers.write().unwrap();
        if let Entry::Occupied(entry) = inactive_map.entry(peer) {
            map.insert(peer, *entry.get());
            entry.remove();
        }
        match map.entry(peer) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().last_seen = Instant::now();
                false
            },
            Entry::Vacant(entry) => {
                if check_addr(peer) {
                    entry.insert(PeerInfo::default());
                    true
                } else {
                    false
                }
            }
        }
    }

    pub fn prune_peers(&self) -> usize {
        let mut inactive_map = self.inactive_peers.write().unwrap();
        let mut map = self.peers.write().unwrap();
        let mut to_prune = Vec::new();
        for (addr, info) in map.iter() {
            if Instant::now() - info.last_seen > Duration::from_secs(KEEPALIVE_CUTOFF) {
                to_prune.push(*addr);
                inactive_map.insert(*addr, *info);
            }
        }
        for addr in to_prune.iter() {
            map.remove(addr);
        }
        to_prune.len()
    }

    pub fn random_peers(&self, n: usize) -> Vec<SocketAddrV6> {
        let mut rng = rand::thread_rng();
        let peers = self.peers.read().unwrap();
        (0..n).into_iter().map(|_| {
            let idx = rng.gen_range::<usize>(0, peers.len());
            peers.get_index(idx).unwrap().0.clone()
        }).collect()
    }
}

fn process_messages<S>(network: NetworkKind, state: Arc<State>, stream: S) -> impl Stream<Item=(Message, SocketAddr), Error=Error>
    where S: Stream<Item=(Message, SocketAddr), Error=Error>
{
    stream.map(move |(mut msg, addr)| -> Box<Stream<Item=(Message, SocketAddr), Error=Error> + Send> {
        if network == msg.header.network {
            let state = state.clone();
            let kind = msg.kind();
            let _ = state.add_or_update_peer(to_ipv6(addr), true);
            debug!("Received message of kind: {:?} from {}", kind, addr);
            match msg.payload {
                MessagePayload::Publish(ref mut block) | MessagePayload::ConfirmReq(ref mut block) => {
                    let hash = match block.hash(false) {
                        Ok(hash) => {
                            hash.into()
                        },
                        Err(e) => {
                            format!("Error calculating hash for block: {}", e)
                        }
                    };
                    info!("Got {:?} block with hash {}, data: {:?}", block.kind, hash, block);
                },
                _ => {}
            }
            match kind {
                MessageKind::KeepAlive => {
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
                },
                _ => {
                    Box::new(stream::empty())
                }
            }
        } else {
            debug!("Received message from {:?} network, ignoring...", msg.header.network);
            Box::new(stream::empty())
        }
    })
    .flatten()
}

fn send_keepalives(state: Arc<State>, timer: &Timer) -> impl Stream<Item=(Message, SocketAddr), Error=Error> {
    stream::once(Ok(()))
        .chain(timer.interval(Duration::from_secs(KEEPALIVE_INTERVAL)))
        .map(move |_| {
            let state = state.clone();
            let count = state.peer_count();
            debug!("Sending keepalives to peers. Current peer count: {}", count);
            let peers = state.peers.read().unwrap().clone();
            let inner_state = state.clone();
            stream::iter_ok::<_, Error>(peers.into_iter()).map(move |(addr, _)| {
                let send_peers = inner_state.random_peers(8);
                let msg = MessageBuilder::new(MessageKind::KeepAlive)
                    .with_payload(MessagePayload::KeepAlive(send_peers))
                    .build();
                (msg, SocketAddr::V6(addr))
            })
        })
        .flatten()
}

fn prune_peers(state: Arc<State>, timer: &Timer) -> impl Future<Item=(), Error=TimerError> {
    timer.interval(Duration::from_secs(PEER_PRUNE_INTERVAL))
        .for_each(move |_| {
            let state = state.clone();
            let count = state.prune_peers();
            debug!("Pruned {} inactive peers. Current peer count: {}", count, state.peer_count());
            futures::future::ok(())
        })
}

pub struct NodeConfig {
    pub peers: Vec<SocketAddr>,
    pub listen_addr: SocketAddr,
    pub network: NetworkKind,
}


pub fn run(config: NodeConfig, handle: &tokio::reactor::Handle) -> Result<impl Future<Item = (), Error = ()>> {
    let socket_std = UdpBuilder::new_v6()?
        .only_v6(false)?
        .bind(&config.listen_addr)?;
    let socket = UdpSocket::from_std(socket_std, handle)?;

    info!("Listening on: {}", socket.local_addr()?);

    let (sink, stream) = UdpFramed::new(socket, MessageCodec::new()).split();

    let initial_peers: IndexMap<SocketAddrV6, PeerInfo> = config.peers.into_iter()
        .map(|addr| {
            (to_ipv6(addr), PeerInfo::default())
        }).collect();

    let state = Arc::new(State::new(initial_peers));

    let message_processor = process_messages(config.network, state.clone(), stream);

    let timer = Timer::default();
    let keepalive_handler = send_keepalives(state.clone(), &timer);
    let peer_prune_handler = prune_peers(state.clone(), &timer);

    let (sock_send, sock_recv) = mpsc::channel::<(nano_lib_rs::message::Message, SocketAddr)>(2048);
    let process_send = sock_send.clone();
    let keepalive_send = sock_send.clone();
    
    Ok(futures::future::lazy(||{
        tokio::spawn(
            process_send
                .sink_map_err(|e| error!("Fatal error sending messages: {:?}", e))
                .send_all(log_errors(message_processor)
                    .map_err(|e| error!("Fatal error processing keepalives: {:?}", e)))
                .map(|_| ())
        );

        tokio::spawn(
            keepalive_send
                .sink_map_err(|e| error!("Fatal sending keepalive: {:?}", e))
                .send_all(log_errors(keepalive_handler)
                    .map_err(|e| error!("Fatal error processing keepalives: {:?}", e)))
                .map(|_| ())
        );

        tokio::spawn(
            peer_prune_handler
                .map_err(|e| error!("Error pruning peers: {}", e))
        );

        tokio::spawn(sink
            .sink_map_err(|e| error!("Fatal error sending message: {:?}", e))
            .send_all(sock_recv)
            .map(|_| ()));

        Ok(())
    }))
}