use nanopow_rs::InputHash;
use super::hash::{Hash, Hasher};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrivateKey([u8; 32]);

impl AsRef<[u8]> for PrivateKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicKey([u8; 32]);

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<PrivateKey> for PublicKey {
    fn from(_key: PrivateKey) -> Self {
        unimplemented!();
    }
}

impl From<PublicKey> for InputHash {
    fn from(key: PublicKey) -> InputHash {
        InputHash::new(key.0)
    }
}

impl Hash for PublicKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(&self.0)
    }
}

pub struct Address(pub String);

impl From<PublicKey> for Address {
    fn from(_key: PublicKey) -> Self {
        unimplemented!();
    }
}

pub struct KeyPair {
    pub private_key: PrivateKey,
    pub public_key: PublicKey,
}

impl From<PrivateKey> for KeyPair {
    fn from(key: PrivateKey) -> Self {
        KeyPair {
            private_key: key.clone(),
            public_key: key.into(),
        }
    }
}

pub struct Account {
    pub public_key: PublicKey,
    pub address: Address,
}

impl From<PublicKey> for Account {
    fn from(key: PublicKey) -> Self {
        Account {
            public_key: key.clone(),
            address: key.into(),
        }
    }
}
