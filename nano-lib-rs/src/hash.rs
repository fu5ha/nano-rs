use super::error::Result;
use std::mem;
use byteorder::{ByteOrder, LittleEndian};

pub trait Hasher {
    type Output;

    fn write(&mut self, bytes: &[u8]);
    fn write_u128(&mut self, i: u128) {
        let mut buf = [0u8; 64];
        LittleEndian::write_u128(&mut buf, i);
        self.write(&buf);
    }
    fn finish(self) -> Result<Self::Output>;
}

pub trait Hash {
    fn hash<H: Hasher>(&self, state: &mut H);
    fn hash_slice<H: Hasher>(data: &[Self], state: &mut H)
    where
        Self: Sized,
    {
        for piece in data {
            piece.hash(state);
        }
    }
}

impl Hash for u128 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u128(*self);
    }
}
