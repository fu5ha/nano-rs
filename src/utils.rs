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

const IPV4_RESERVED_ADDRESSES: &[(u32, u32)] = &[
    (0x00000000, 0x00ffffff), // rfc 1700
    (0x7f000000, 0x7fffffff), // loopback
    (0xc0000200, 0xc00002ff), // rfc 5737
    (0xc6336400, 0xc63364ff), // rfc 5737
    (0xcb007100, 0xcb0071ff), // rfc 5737
    (0xe0000000, 0xefffffff), // multicast
    (0xf0000000, 0xffffffff), // rfc 6890
];

use std::net::SocketAddrV6;

pub fn check_addr(addr: SocketAddrV6) -> bool {
    let ip = addr.ip().clone();
    if ip.octets().iter().all(|&x| x == 0) {
        return false;
    }
    if addr.port() == 0 {
        return false;
    }
    if ip.is_unspecified() || ip.is_loopback() || ip.is_multicast()
    {
        return false;
    }
    if let Some(ip) = ip.to_ipv4() {
        let ip: u32 = ip.into();
        for &(start, end) in IPV4_RESERVED_ADDRESSES.iter() {
            if ip >= start && ip <= end {
                return false;
            }
        }
    }
    true
}
