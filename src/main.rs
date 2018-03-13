#![feature(conservative_impl_trait)]
extern crate tokio;
extern crate tokio_io;
extern crate futures;

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

use nano_lib_rs::message::{MessageBuilder, MessageKind, MessageCodec};

use tokio::prelude::*;
use tokio::net::{UdpSocket, UdpFramed};
use futures::Future;

use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::{Arc, Mutex};

use bytes::Bytes;

struct State {
    pub peers: Mutex<Vec<SocketAddr>>
}

impl State {
    pub fn new() -> Self {
        State {
            peers: Mutex::new(Vec::new())
        }
    }

    pub fn add_peer(&self, peer: SocketAddr) -> usize {
        let mut peers = self.peers.lock().unwrap();
        let is_present = peers.iter().any(|&p| p == peer);
        if !is_present {
            peers.push(peer);
        }
        peers.len()
    }
}

fn run() -> Result<()> {
    info!("Starting nano-rs!");

    let addr = "0.0.0.0:7075".parse()?;
    let socket = UdpSocket::bind(&addr)?;

    info!("Listening on: {}", socket.local_addr()?);

    let (sink, stream) = UdpFramed::new(socket, MessageCodec::new()).split();

    let init_addrs = "rai.raiblocks.net:7075".to_socket_addrs()?;
    let mut initial_peers = Vec::new();
    for addr in init_addrs {
        if let SocketAddr::V4(_) = addr {
            info!("Found initial peer: {}", addr);
            initial_peers.push(addr);
        }
    }

    if let None = initial_peers.get(0) {
        return Err("Could not connect to initial peer".into());
    }

    let state = Arc::new(State::new());

    let init_msgs = stream::iter_ok::<_, nano_lib_rs::error::Error>(initial_peers.into_iter()).map(|peer| {
        info!("Sending keepalive to initial peer: {}", peer);
        let fake_data = [0u8; 144];
        let msg = MessageBuilder::new(MessageKind::KeepAliveMessage)
            .with_data(Bytes::from(&fake_data[..]))
            .build();
        (msg, peer)
    });
    let handler = sink.send_all(init_msgs)
        .and_then(move |(_sink, _source_stream)| {
            let state = state.clone();
            stream.for_each(move |content| {
                let kind = content.0.kind();
                let count = state.add_peer(content.1);
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
        .chain(std::io::stdout())
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
