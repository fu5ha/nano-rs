pub mod handler;
pub mod state;
use self::state::{State, PeerInfo};

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
use std::sync::{Arc};

use tokio_timer::{Timer, TimerError};
use std::time::{Duration};

use indexmap::IndexMap;

use error::*;

use utils::{log_errors, to_ipv6};

const KEEPALIVE_INTERVAL: u64 = 60;
const KEEPALIVE_CUTOFF: u64 = KEEPALIVE_INTERVAL * 5;

const PEER_PRUNE_INTERVAL: u64 = KEEPALIVE_INTERVAL * 2;

fn process_messages<S>(network: NetworkKind, state: Arc<State>, stream: S) -> impl Stream<Item=(Message, SocketAddr), Error=Error>
    where S: Stream<Item=(Message, SocketAddr), Error=Error>
{
    stream.map(move |(msg, src_addr)| -> Box<Stream<Item=(Message, SocketAddr), Error=Error> + Send> {
        if network == msg.header.network {
            let state = state.clone();
            let kind = msg.kind();
            let src_addr_v6 = to_ipv6(src_addr);
            let _ = state.add_or_update_peer(src_addr_v6, true);
            debug!("Received message of kind: {:?} from {}", kind, src_addr);
            match kind {
                MessageKind::KeepAlive => handler::keepalive(msg, src_addr_v6, state.clone()),
                MessageKind::Publish => handler::publish(msg, src_addr_v6, state.clone()),
                MessageKind::ConfirmReq => handler::confirm_req(msg, src_addr_v6, state.clone()),
                _ => Box::new(stream::empty())
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

    let initial_peers: IndexMap<SocketAddrV6, PeerInfo> = config.peers.into_iter()
        .map(|addr| {
            (to_ipv6(addr), PeerInfo::default())
        }).collect();

    let state = Arc::new(State::new(initial_peers));

    let (sink, stream) = UdpFramed::new(socket, MessageCodec::new(), state.clone()).split();

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