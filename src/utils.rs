use std::marker::PhantomData;

use futures::{Async, Stream};
use error::*;

#[macro_export]
macro_rules! default_addr {
    () => { ::std::net::SocketAddrV6::new(::std::net::Ipv6Addr::from([0u8; 16]), 0, 0, 0) };
}

pub struct LogErrors<S: Stream, I> {
    inner: S,
    phantom_item: PhantomData<I>
}

impl<S, I> Stream for LogErrors<S, I> 
    where S: Stream<Item = I, Error = Error>
{
    type Item = S::Item;
    type Error = Error;

    fn poll(&mut self) -> Result<Async<Option<S::Item>>> {
        loop {
            match self.inner.poll() {
                Ok(x) => return Ok(x),
                Err(e) => {
                    if let ErrorKind::FatalStreamError = *e.kind() {
                        return Err(e);
                    } else {
                        error!("Non-fatal error in stream: {:?}", e);
                    }
                },
            }
        }
    }
}

pub fn log_errors<S, I>(stream: S) -> LogErrors<S, I> 
    where S: Stream<Item = I, Error = Error>
{
    LogErrors {
        inner: stream,
        phantom_item: PhantomData,
    }
}