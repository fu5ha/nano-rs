#![feature(conservative_impl_trait)]
extern crate tokio;
extern crate tokio_io;
extern crate tokio_timer;
extern crate net2;
#[macro_use]
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

extern crate rand;
extern crate indexmap;

mod error;
mod net;
mod utils;
mod node;

use error::*;
use node::{NodeConfig};

use nano_lib_rs::message::NetworkKind;

use std::net::{ToSocketAddrs, SocketAddr};

use futures::{Future};

fn run() -> Result<()> {
    info!("Starting nano-rs!");

    let listen_addr = "[::]:7075".parse()?;
    let peers: Vec<SocketAddr> = "rai.raiblocks.net:7075".to_socket_addrs()?.collect();
    if let None = peers.get(0) {
        return Err("Could not connect to initial peer".into());
    }
    let network = NetworkKind::Main;

    let config = NodeConfig {
        peers,
        network,
        listen_addr,
    };

    let mut runtime = tokio::runtime::Runtime::new()?;
    let handle = runtime.handle().clone();
    let node = node::run(config, &handle)?;

    runtime.spawn(node);
    runtime.shutdown_on_idle().wait().unwrap();

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
        .level(log::LevelFilter::Info)
        .level_for("tokio_reactor", log::LevelFilter::Error)
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
