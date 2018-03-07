use nanopow_rs;
use hex;

error_chain!{
    errors {
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
    }

    links {
        NanopowError(nanopow_rs::error::Error, nanopow_rs::error::ErrorKind) #[doc = "An error occurred while generating Proof of Work."];
    }

    foreign_links {
        FromHexConversionError(hex::FromHexError) #[doc = "An error occurred while converting from hex"];
        FormatError(::std::fmt::Error) #[doc = "A formatting error occured"];
    }
}