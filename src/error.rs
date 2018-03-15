
error_chain!{
    errors{
        /// A non recoverable error occurred while processing a stream
        #[allow(dead_code)]
        FatalStreamError {
            description("A non recoverable error occurred while processing a stream")
            display("A non recoverable error occurred while processing a stream")
        }
        /// An error occurred with a Tokio-timer timeout
        TokioTimeoutError(inner: String) {
            description("Error in Tokio Timeout")
            display("Error in tokio timeout: {}", inner)
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
        TokioTimerError(::tokio_timer::TimerError) #[doc = "An error occurred in a tokio timer"];
    }
}

impl<T> From<::tokio_timer::TimeoutError<T>> for Error {
    fn from(err: ::tokio_timer::TimeoutError<T>) -> Self {
        Self::from_kind(ErrorKind::TokioTimeoutError(format!("{}", err)))
    }
}
