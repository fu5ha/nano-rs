#![recursion_limit = "1024"]
#![feature(i128_type)]
#[macro_use]
extern crate error_chain;

#[macro_use]
extern crate bitflags;

extern crate blake2;
extern crate bytes;
extern crate data_encoding;

extern crate nanopow_rs;

pub mod block;
pub mod keys;
pub mod hash;
pub mod error;
pub mod message;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
