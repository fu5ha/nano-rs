pub trait Hasher {
    type Output;
    fn write(&mut self, bytes: &[u8]);
    fn finish(&self) -> Output;
}

pub trait Hash {
    fn hash<H: Hasher>(&self, state: &mut H);
}
