use super::error::Result;
use std::mem;

pub trait Hasher {
    type Output;

    fn write(&mut self, bytes: &[u8]);
    fn write_u128(&mut self, i: u128) {
        self.write(&unsafe { mem::transmute::<_, [u8; 16]>(i) })
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
