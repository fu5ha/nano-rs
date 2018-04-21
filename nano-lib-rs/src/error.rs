error_chain!{
    errors {
        /// Invalid character used in hex string
        InvalidHexCharacterError(pos: usize) {
            description("invalid character in hex")
            display("Invalid character in hex string at position {}", pos)
        }
        /// Attempted to set invalid work for a block
        InvalidWorkError {
            description("invalid work")
            display("Invalid Work")
        }
        /// Attempted to verify work for block with no work
        NoWorkError {
            description("attempted to verify work for block with no work")
            display("Attempted to verify work for block with no work")
        }
        /// Attempted to create a BlockHash with incorrect length
        BlockHashLengthError {
            description("attempted to create BlockHash with invalid length")
            display("Attempted to create BlockHash with invalid length")
        }
        /// Attempted to create a Signature with incorrect length
        SignatureLengthError {
            description("attempted to create Signature with invalid length")
            display("Attempted to create Signature with invalid length")
        }
        /// Attempted to create or parse a message with invalid header length
        MessageHeaderLengthError(len: usize) {
            description("Attempted to create or parse a message with invalid header length")
            display("Attempted to create message with buffer of length {} (must be at least 8)", len)
        }
        /// Error while parsing block
        BlockParseError(kind: BlockParseErrorKind) {
            description("Error while parsing block")
            display("Error while parsing block: {:?}", kind)
        }
        /// Attempted to create or parse a block with invalid data length for its kind
        BlockPayloadLengthError(kind: super::block::BlockKind, len: usize) {
            description("Attempted to create or parse a block with invalid data length for its kind")
            display("Attempted to create block of type {:?} with data length {} (should be {})", kind, len, kind.size())
        }
        /// Attempted to deserialize a block payload for a type that does not have a payload
        InvalidBlockPayloadKindError(kind: super::block::BlockKind) {
            description("Attempted to deserialize a block payload for a type that does not have a payload")
            display("Attempted to deserialize a block payload for type {:?}, which does not have a payload", kind)
        }
        /// An error occurred while decoding an ed25519 key
        EdwardsDecodingError(err: String) {
            description("An error occurred while decoding an ed25519 key")
            display("{}", err)
        }
        /// Attempted to decode message with invalid magic number
        InvalidMagicNumber {
            description("Invalid magic number")
            display("Invalid magic number")
        }

		SeedLengthError(len: usize) {
			description("Invalid Seed Length")
			display("Invalid Seed Length! Expected 64 Got {}", len)
		}

		InvalidAddress {
			description("Invalid Address")
			display("Invalid Address")
		}

		InvalidAddressLength(len: usize) {
			description("Invalid Address Length")
			display("Invalid Address Length! Expected 64 Got {}", len)
		}
    }

    links {
        NanopowError(::nanopow_rs::error::Error, ::nanopow_rs::error::ErrorKind) #[doc = "An error occurred while generating Proof of Work."];
    }

    foreign_links {
		DecodeError(::data_encoding::DecodeError);
        FormatError(::std::fmt::Error) #[doc = "A formatting error occured"];
        BincodeError(::bincode::Error) #[doc = "An error occurred while serializing/deserializing binary data."];
        IoError(::std::io::Error) #[doc = "An IO error occurred"];
    }
}

#[derive(Debug, Copy, Clone)]
pub enum BlockParseErrorKind {
    NoSignature,
    NoWork,
}

impl From<::ed25519_dalek::DecodingError> for Error {
    fn from(err: ::ed25519_dalek::DecodingError) -> Self {
        Self::from_kind(ErrorKind::EdwardsDecodingError(format!("{}", err)))
    }
}
