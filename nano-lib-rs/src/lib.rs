#![recursion_limit = "1024"]
#![feature(i128_type)]
#![allow(unused_imports)]
#[macro_use]
extern crate error_chain;

extern crate byteorder;

#[macro_use]
extern crate log;

#[macro_use]
extern crate bitflags;

extern crate bincode;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

extern crate blake2;
extern crate bytes;
extern crate data_encoding;

extern crate nanopow_rs;

extern crate tokio_io;

#[macro_use]
mod macros; 

pub mod block;
pub mod keys;
pub mod hash;
pub mod error;
pub mod message;
