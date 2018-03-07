#![recursion_limit = "1024"]
#[macro_use]
extern crate error_chain;

extern crate blake2;
extern crate bytes;
extern crate hex;

extern crate nanopow_rs;

pub mod block;
pub mod keys;
pub mod hash;
pub mod error;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
