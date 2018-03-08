
error_chain! {
  errors {
    /// Invalid character used in hex string
    InvalidHexCharacterError(pos: usize) {
      description("invalid character in hex")
      display("Invalid character in hex string at position {}", pos)
    }
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
    FormatError(::std::fmt::Error) #[doc = "A formatting error occured"];
  }
}