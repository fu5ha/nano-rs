use super::hash::{Hash, Hasher};
use blake2::{
	digest::{Input, VariableOutput},
    Blake2b
};
use byteorder::{BigEndian, WriteBytesExt};
use data_encoding::{Encoding, HEXUPPER};
pub use ed25519_dalek::{
	Keypair,
	PublicKey,
	SecretKey,
	Signature,
	PUBLIC_KEY_LENGTH,
    SECRET_KEY_LENGTH,
	SIGNATURE_LENGTH
};
use error::*;
use std::ops::{Deref, DerefMut};

impl Hash for PublicKey {
	fn hash<H: Hasher>(&self, state: &mut H) {
		state.write(self.as_bytes())
	}
}

const XRB_ENCODING: Encoding = new_encoding! {
	symbols: "13456789abcdefghijkmnopqrstuwxyz",
	check_trailing_bits: false,
};

#[derive(Debug, Clone)]
pub struct Address(pub String);

#[derive(Clone)]
pub struct Seed(pub [u8; 32]);

#[derive(Debug)]
pub struct PrivateKey(SecretKey);

impl Seed {
	pub fn from<T: AsRef<[u8]>>(seed: T) -> Result<Self> {
		let seed = seed.as_ref();
		if seed.len() != 64 {
			bail!(ErrorKind::SeedLengthError(seed.len()))
		}

		let seed = HEXUPPER.decode(&seed).unwrap();

		let mut seed_bytes = [0u8; 32];
		seed_bytes.copy_from_slice(&seed);

		Ok(Seed(seed_bytes))
	}
}

impl Deref for Seed {
	type Target = [u8; 32];
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for Seed {
	fn deref_mut(&mut self) -> &mut [u8; 32] {
		&mut self.0
	}
}

impl Address {
	pub fn to_public_key(&self) -> Result<PublicKey> {
		if let Some("xrb_") = self.0.get(..4) {
			if self.0.len() == 64 {
				let mut encoded_addr = String::from(self.0.get(4..56).unwrap());
				encoded_addr.insert_str(0, "1111");
				let checksum = self.0.get(56..).unwrap();
				let pkey_bytes = XRB_ENCODING.decode(encoded_addr.as_bytes())?;
				let derived_checksum =
					XRB_ENCODING.encode(&compute_address_checksum(&pkey_bytes[3..]));
				if checksum != derived_checksum {
					bail!(ErrorKind::InvalidAddress)
				}
				return Ok(PublicKey::from_bytes(&pkey_bytes[3..])?);
			}
			bail!(ErrorKind::InvalidAddressLength(self.0.len()));
		}
		bail!(ErrorKind::InvalidAddress)
	}
}

/// the address checksum is the 5byte hash of the public key reversed
///
pub fn compute_address_checksum(key_bytes: &[u8]) -> [u8; 5] {
	let mut blake = Blake2b::new(5).unwrap();
	let mut buf = [0u8; 5];
	blake.process(key_bytes);
	blake.variable_result(&mut buf).unwrap();
	buf.reverse();
	buf
}

#[derive(Debug, Clone)]
pub struct Account {
	pub public_key: PublicKey,
	pub address: Address,
}

impl From<PublicKey> for Address {
	fn from(key: PublicKey) -> Self {
		let mut p_key = key.to_bytes().to_vec();
		let mut h = [0u8; 3].to_vec();
		h.append(&mut p_key);
		let checksum = XRB_ENCODING.encode(&compute_address_checksum(key.as_bytes()));
		let address = {
			let encoded_addr = XRB_ENCODING.encode(&h);
			let mut addr = String::from("xrb_");
			addr.push_str(encoded_addr.get(4..).unwrap());
			addr.push_str(&checksum);
			addr
		};

		Address(address)
	}
}

impl From<PublicKey> for Account {
	fn from(key: PublicKey) -> Self {
		Account {
			public_key: key.clone(),
			address: key.into(),
		}
	}
}

impl From<PrivateKey> for Account {
	fn from(key: PrivateKey) -> Self {
		let public_key: PublicKey = key.into();
		Account {
			public_key: public_key.clone(),
			address: public_key.into(),
		}
	}
}

impl From<PrivateKey> for PublicKey {
	fn from(key: PrivateKey) -> Self {
		PublicKey::from_secret::<Blake2b>(&key)
	}
}

impl Deref for PrivateKey {
	type Target = SecretKey;
	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl DerefMut for PrivateKey {
	fn deref_mut(&mut self) -> &mut SecretKey {
		&mut self.0
	}
}

impl PrivateKey {
	pub fn from_seed(seed: Seed, index: u32) -> PrivateKey {
		let mut blake = Blake2b::new(32).unwrap();
		let mut index_buf = Vec::with_capacity(4);
		index_buf.write_u32::<BigEndian>(index).unwrap();
		blake.process(&*seed);
		blake.process(&index_buf);

		let mut buf = [0u8; 32];
		blake.variable_result(&mut buf).unwrap();
		PrivateKey(SecretKey::from_bytes(&buf).unwrap())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn can_generate_address_from_seed() {
		let seed = Seed::from("1234567890123456789012345678901234567890123456789012345678901234").unwrap();

		// shamelessly copied from https://github.com/frankh/nano/blob/078a99b8e75bd239e13565312e06258164a781d5/address/address_test.go#L55-L59
		let expected_output = vec![
			"xrb_3iwi45me3cgo9aza9wx5f7rder37hw11xtc1ek8psqxw5oxb8cujjad6qp9y",
			"xrb_3a9d1h6wt3zp8cqd6dhhgoyizmk1ciemqkrw97ysrphn7anm6xko1wxakaa1",
			"xrb_1dz36wby1azyjgh7t9nopjm3k5rduhmntercoz545my9s8nm7gcuthuq9fmq",
			"xrb_1fb7kaqaue49kf9w4mb9w3scuxipbdm3ez6ibnri4w8qexzg5f4r7on1dmxb",
			"xrb_3h9a64yqueuij1j9odt119r3ymm8n83wyyz7o9u7ram1tgfhsh1zqwjtzid9",
		];

		expected_output
			.into_iter()
			.enumerate()
			.for_each(|(index, address)| {
				let priv_key = PrivateKey::from_seed(seed.clone(), index as u32);
				let account: Account = priv_key.into();

				assert_eq!(account.address.0, address)
			})
	}

	#[test]
	fn can_convert_address_to_public_key() {
		let addr =
			Address("xrb_3t6k35gi95xu6tergt6p69ck76ogmitsa8mnijtpxm9fkcm736xtoncuohr3".into());
		let public_key = addr.to_public_key().unwrap();
		let p_key_str = HEXUPPER.encode(public_key.as_bytes());
		// shamelessly copied from https://github.com/frankh/nano/blob/078a99b8e75bd239e13565312e06258164a781d5/address/address_test.go#L28-L30
		assert_eq!(
			p_key_str,
			"e89208dd038fbb269987689621d52292ae9c35941a7484756ecced92a65093ba".to_uppercase()
		)
	}

	#[test]
	fn can_validate_addresses() {
		let addresses = vec![
			"xrb_38nm8t5rimw6h6j7wyokbs8jiygzs7baoha4pqzhfw1k79npyr1km8w6y7r8",
			"xrb_1awsn43we17c1oshdru4azeqjz9wii41dy8npubm4rg11so7dx3jtqgoeahy",
			"xrb_3arg3asgtigae3xckabaaewkx3bzsh7nwz7jkmjos79ihyaxwphhm6qgjps4",
			"xrb_3pczxuorp48td8645bs3m6c3xotxd3idskrenmi65rbrga5zmkemzhwkaznh",
			"xrb_3hd4ezdgsp15iemx7h81in7xz5tpxi43b6b41zn3qmwiuypankocw3awes5k",
			"xrb_1anrzcuwe64rwxzcco8dkhpyxpi8kd7zsjc1oeimpc3ppca4mrjtwnqposrs",
		];

		addresses.into_iter().for_each(|addr| {
			Address(addr.into())
				.to_public_key()
				.expect("Couldn't Validate Address");
		})
	}

	#[test]
	fn can_invalidate_addresses() {
		let addresses = vec![
			"xrb_38nm8t5rimw6h6j7wyokbs8jiygzs7baoha4pqzhfw1k79npyr1km8w6y7r7",
			"xrc_38nm8t5rimw6h6j7wyokbs8jiygzs7baoha4pqzhfw1k79npyr1km8w6y7r8",
			"xrb38nm8t5rimw6h6j7wyokbs8jiygzs7baoha4pqzhfw1k79npyr1km8w6y7r8",
			"xrb8nm8t5rimw6h6j7wyokbs8jiygzs7baoha4pqzhfw1k79npyr1km8w6y7r8",
			"xrb_8nm8t5rimw6h6j7wyokbs8jiygzs7baoha4pqzhfw1k79npyr1km8w6y7r8",
		];

		let output = addresses
			.into_iter()
			.map(|addr| Address(addr.into()).to_public_key().is_err())
			.collect::<Vec<_>>();

		assert_eq!(output, vec![true, true, true, true, true])
	}
}
