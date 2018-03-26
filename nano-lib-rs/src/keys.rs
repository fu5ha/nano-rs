use nanopow_rs::InputHash;
use super::hash::{Hash, Hasher};

use serde::{Serialize, Deserialize};

pub use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH, SIGNATURE_LENGTH};

impl Hash for PublicKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(self.as_bytes())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Address(pub String);

impl From<PublicKey> for Address {
    fn from(_key: PublicKey) -> Self {
        unimplemented!();
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
