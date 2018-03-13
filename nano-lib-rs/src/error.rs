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
        /// Attempted to create or parse a message with invalid data length for its kind
        MessageDataLengthError(kind: super::message::MessageKind, len: usize) {
            description("Attempted to create or parse a message with invalid data length for its kind")
            display("Attempted to create message of type {:?} with data length {} (should be {})", kind, len, kind.size().unwrap())
        }
        /// Attempted to create or parse a message with invalid header length
        MessageHeaderLengthError(len: usize) {
            description("Attempted to create or parse a message with invalid header length")
            display("Attempted to create message with buffer of length {} (must be at least 8)", len)
        }
    }

    links {
        NanopowError(::nanopow_rs::error::Error, ::nanopow_rs::error::ErrorKind) #[doc = "An error occurred while generating Proof of Work."];
    }

    foreign_links {
        FormatError(::std::fmt::Error) #[doc = "A formatting error occured"];
        BincodeError(::bincode::Error) #[doc = "An error occurred while serializing/deserializing binary data."];
        IoError(::std::io::Error) #[doc = "An IO error occurred"];
    }
}
