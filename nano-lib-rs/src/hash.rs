use super::error::Result;

pub trait Hasher {
    type Output;
   
    fn write(&mut self, bytes: &[u8]);
    fn finish(self) -> Result<Self::Output>;
}

pub trait Hash {
    fn hash<H: Hasher>(&self, state: &mut H);
    fn hash_slice<H: Hasher>(data: &[Self], state: &mut H) 
        where Self: Sized
    {
        for piece in data {
            piece.hash(state);
        }
    }
}
