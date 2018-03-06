#[feature(associated_consts)]
#[macro_use]
extern crate error_chain

extern crate blake2;
extern crate bytes;

pub mod block;
pub mod keys;
pub mod hash;
pub mod errors;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
