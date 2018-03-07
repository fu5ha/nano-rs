use hex;

error_chain! {
  errors {
    /// Attempted to create an InputHash with incorrect length
    HashLengthError {
      description("attempted to create InputHash with invalid length")
      display("Attempted to create InputHash with invalid length")
    }

    /// Attempted to create Work with incorrect length
    WorkLengthError {
      description("attempted to create Work with invalid length")
      display("Attempted to create Work with invalid length")
    }
  }

  foreign_links {
    FromHexConversionError(hex::FromHexError) #[doc = "An error occurred while converting from hex"];
    FormatError(::std::fmt::Error) #[doc = "A formatting error occured"];
  }
}