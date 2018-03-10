extern crate tokio;
extern crate futures;
extern crate tokio_io;

extern crate nano_lib_rs;

#[macro_use]
extern crate log;
extern crate fern;
extern crate chrono;

#[macro_use]
extern crate error_chain;

mod error;
use error::*;

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

fn run() -> Result<()> {
    info!("Starting nano-rs!");

    info!("Stopping nano-rs!");
    Ok(())
}
