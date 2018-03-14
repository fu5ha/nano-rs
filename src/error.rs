
error_chain!{
    errors{
        /// A recoverable error occurred while processing a stream
        RecoverableStreamError {
            description("A recoverable error occurred while processing a stream")
            display("A recoverable error occurred while processing a stream")
        }
        /// A non recoverable error occurred while processing a stream
        NonRecoverableStreamError {
            description("A non recoverable error occurred while processing a stream")
            display("A non recoverable error occurred while processing a stream")
        }
    }
    links{
        NanoLibError(::nano_lib_rs::error::Error, ::nano_lib_rs::error::ErrorKind) #[doc = "An error occurred in nano-lib"];
    }
    foreign_links{
        FernInitError(::fern::InitError) #[doc = "An error occured while setting up fern"];
        SetLoggerError(::log::SetLoggerError) #[doc = "An error occured while setting the logger"];
        IoError(::std::io::Error) #[doc = "An IO error occurred"];
        AddrParseError(::std::net::AddrParseError) #[doc = "An error occurred while parsing an address"];
    }
}
