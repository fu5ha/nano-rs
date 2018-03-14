extern crate tokio;
extern crate tokio_io;
extern crate futures;

extern crate data_encoding;

extern crate nano_lib_rs;

#[macro_use]
extern crate log;
extern crate fern;
extern crate chrono;

#[macro_use]
extern crate error_chain;

extern crate bytes;

mod error;
use error::*;
mod net;
use net::message_codec::MessageCodec;

use nano_lib_rs::message::{MessageBuilder, MessageKind, MessageInner};

use tokio::prelude::*;
use tokio::net::{UdpSocket, UdpFramed};
use futures::Future;

use std::net::{SocketAddr, ToSocketAddrs, SocketAddrV6};
use std::sync::{Arc, Mutex};

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

fn run() -> Result<()> {
    info!("Starting nano-rs!");

    let addr = "[::]:7075".parse()?;
    let socket = UdpSocket::bind(&addr)?;

    info!("Listening on: {}", socket.local_addr()?);

    let (sink, stream) = UdpFramed::new(socket, MessageCodec::new()).split();

    let init_addrs = "rai.raiblocks.net:7075".to_socket_addrs()?;
    let mut init_peers_v6 = Vec::new();
    let initial_peers: Vec<SocketAddrV6> = init_addrs
        // TODO: Handle send errors so we can use ipv6 confidently
        .filter(|&addr| {
            init_peers_v6.push(to_ipv6(addr));
            match addr {
                SocketAddr::V6(_) => false,
                SocketAddr::V4(_) => true
            }
        }).map(|addr| {
            to_ipv6(addr)
        }).collect();

    if let None = initial_peers.get(0) {
        return Err("Could not connect to initial peer".into());
    }

    let state = Arc::new(State::new());

    let init_msgs = stream::iter_ok::<_, nano_lib_rs::error::Error>(initial_peers.into_iter()).map(move |peer| {
        info!("Sending keepalive to initial peer: {}", peer);
        let msg = MessageBuilder::new(MessageKind::KeepAlive)
            .with_data(MessageInner::KeepAlive(init_peers_v6.clone()))
            .build();
        (msg, SocketAddr::V6(peer))
    });
    let handler = sink.send_all(init_msgs)
        .and_then(move |(_sink, _source_stream)| {
            let state = state.clone();
            stream.for_each(move |content| {
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
        })
        .map(|_| ())
        .map_err(|e| error!("Got error: {:?}", e));

    tokio::run(
        handler
    );

    info!("Stopping nano-rs!");
    Ok(())
}

fn setup_logger() -> Result<()> {
    use std::fs::create_dir;
    let base_path: &str = match create_dir("log") {
        Ok(_) => {
            "log/"
        },
        Err(e) => {
            if e.kind() == std::io::ErrorKind::AlreadyExists {
                "log/"
            } else {
                ""
            }
        }
    };
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "{}[{}][{}] {}",
                chrono::Local::now().format("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
                message
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stderr())
        .chain(fern::log_file(format!("{}nano-rs__{}.log", base_path, chrono::Local::now().format("%Y-%m-%d__%H-%M-%S")))?)
        .apply()?;
    Ok(())
}

fn main() {
    // Setup logger
    if let Err(e) = setup_logger() {
        use std::io::Write;
        let stderr = &mut ::std::io::stderr();
        let errmsg = "Error writing to stderr";

        writeln!(stderr, "Error while initializing logger: {}", e).expect(errmsg);
    }

    // Run program and log errors from error-chain using logger
    if let Err(ref e) = run() {

        error!("Failed with error: {}", e);

        for e in e.iter().skip(1) {
            error!("Caused by: {}", e);
        }

        if let Some(backtrace) = e.backtrace() {
            error!("backtrace: {:?}", backtrace);
        }

        ::std::process::exit(1);
    }
}
