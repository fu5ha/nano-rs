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
extern crate clap;

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

use clap::{Arg, App};

fn run(network: NetworkKind) -> Result<()> {
    info!("Starting nano-rs!");

    // TODO: Figure out why beta doesn't work and add test
    let port = match network {
        NetworkKind::Beta => 54000,
        _ => 7075
    };

    let listen_addr = format!("[::]:{}", port).parse()?;
    let peers: Vec<SocketAddr> = format!("rai.raiblocks.net:{}", port).to_socket_addrs()?.collect();
    if let None = peers.get(0) {
        return Err("Could not connect to initial peer".into());
    }

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

fn setup_logger(log_level: log::LevelFilter) -> Result<()> {
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
        .level(log_level)
        .level_for("tokio_reactor", log::LevelFilter::Error)
        .chain(std::io::stderr())
        .chain(fern::log_file(format!("{}nano-rs__{}.log", base_path, chrono::Local::now().format("%Y-%m-%d__%H-%M-%S")))?)
        .apply()?;
    Ok(())
}

fn main() {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author("Gray Olson <gray@grayolson.com>")
        .about("An implementation of Nano in Rust using Tokio.")
        .arg(Arg::with_name("log-level")
            .short("l")
            .long("log-level")
            .value_name("LOG_LEVEL")
            .default_value("info")
            .possible_values(&["off", "error", "warn", "info", "debug", "trace"])
            .case_insensitive(true)
            .help("Set logging level (default Info)"))
        .arg(Arg::with_name("network")
            .short("n")
            .long("network")
            .value_name("NET")
            .default_value("live")
            .possible_values(&["live", "beta", "test"])
            .help("The nano network to connect to"))
        .get_matches();

    let network = match matches.value_of("network").unwrap() {
        "live" => NetworkKind::Live,
        "beta" => NetworkKind::Beta,
        "test" => NetworkKind::Test,
        _ => unreachable!(),
    };

    let log_level = match matches.value_of("log-level").unwrap() {
        "off" => log::LevelFilter::Off,
        "error" => log::LevelFilter::Error,
        "warn" => log::LevelFilter::Warn,
        "info" => log::LevelFilter::Info,
        "debug" => log::LevelFilter::Debug,
        "trace" => log::LevelFilter::Trace,
        _ => unreachable!(),
    };

    // Setup logger
    if let Err(e) = setup_logger(log_level) {
        use std::io::Write;
        let stderr = &mut ::std::io::stderr();
        let errmsg = "Error writing to stderr";

        writeln!(stderr, "Error while initializing logger: {}", e).expect(errmsg);
    }

    // Run program and log errors from error-chain using logger
    if let Err(ref e) = run(network) {

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
