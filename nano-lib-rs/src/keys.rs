use super::hash::{Hash, Hasher};
use blake2::{
	digest::{Input, VariableOutput},
    Blake2b
};
use data_encoding::Encoding;
pub use ed25519_dalek::{
	Keypair,
	PublicKey,
	SecretKey,
	Signature,
	PUBLIC_KEY_LENGTH,
    SECRET_KEY_LENGTH,
	SIGNATURE_LENGTH
};
use nanopow_rs::InputHash;

impl Hash for PublicKey {
	fn hash<H: Hasher>(&self, state: &mut H) {
		state.write(self.as_bytes())
	}
}

const XRB_ENCODING: Encoding = new_encoding! {
	symbols: "13456789abcdefghijkmnopqrstuwxyz",
};

pub struct Address(pub String);

impl From<PublicKey> for Address {
	fn from(key: PublicKey) -> Self {
		let mut p_key = key.to_bytes().to_vec();
		let mut h = [0u8; 1].to_vec();
		h.append(&mut p_key);
		let checksum = XRB_ENCODING.encode(&compute_address_checksum(key));
		let address = {
			let encoded_addr = XRB_ENCODING.encode(&h);
			let mut addr = String::from("xrb_");
			addr.push_str(encoded_addr.get(1..).unwrap());
			addr.push_str(&checksum);
			addr
		};

		Address(address)
	}
}

/// the address checksum is the 5byte hash of the public key reversed
///
pub fn compute_address_checksum(key: PublicKey) -> [u8; 5] {
	let mut blake = Blake2b::new(5).unwrap();
	let mut buf = [0u8; 5];
	blake.process(key.as_bytes());
	blake.variable_result(&mut buf).unwrap();
	buf.reverse();
	buf
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
