#![feature(underscore_lifetimes)]

extern crate url;
extern crate r2d2;
extern crate futures;
extern crate tokio_core;
extern crate native_tls;
extern crate tokio_tls;

extern crate lastfm_parse_rs as lastfm;
extern crate async_http_client;

// ----------------------------------------------------------------

use std::fmt::Debug;
use std::io::ErrorKind as IoErrorKind;
use std::net::ToSocketAddrs;

use url::Url;

use r2d2::Pool as ConnPool;

use tokio_core::reactor::Handle;

use native_tls::TlsConnector;
use tokio_tls::{TlsConnectorExt, ConnectAsync};

use lastfm::{LastfmType, Request, RequestParams, from_json_str};

use async_http_client::prelude::*;
use async_http_client::HttpRequest;

// ----------------------------------------------------------------

mod pool;
mod utils;

#[cfg(test)]
mod tests;

use pool::TcpStreamManager;
use utils::{Error, Result, Response};

// ----------------------------------------------------------------

macro_rules! request {
    ($client:ident, $url:expr, $method:expr) => {
        match $url.scheme() {
            "http" => Box::new(result($client.get_http_stream()).and_then($method)),
            "https" => {
                if $url.domain().is_none() {
                    Box::new(err(Error::io(IoErrorKind::InvalidInput, "no domain in url")))
                } else {
                    let https = $client.get_https_stream($url.domain().unwrap()).map_err(From::from);
                    Box::new(https.and_then($method))
                }
            }
            _ => Box::new(err(Error::io(IoErrorKind::InvalidInput, "no scheme in url"))),
        }
    }
}

macro_rules! get {
    ($url:ident, $buf:ident) => {
        |stream| {
            result(HttpRequest::get($url).map_err(|e| Error::io(IoErrorKind::Other, e)))
                .and_then(send!(stream, $buf))
        }
    }
}

macro_rules! send {
    ($stream:ident, $buf:ident) => {
        |req| {
            req.send($stream).map_err(From::from).and_then(move |res| {
                if let (Some(resp), _) = res {
                    // serde doesnt support inplace escape sequence decoding yet
                    // (see https://github.com/serde-rs/json/issues/318)
                    *$buf = String::from_utf8_lossy(resp.get_body())
                        .into_owned()
                        .replace("\\\"", "'");

                    let data: Result<T> = from_json_str($buf).map_err(From::from);
                    return result(data);
                }
                err(Error::io(IoErrorKind::UnexpectedEof, "no response body"))
            })
        }
    }
}

// ----------------------------------------------------------------

pub struct Client {
    base_url: String,
    api_key: String,
    handle: Handle,
    pool: ConnPool<TcpStreamManager>,
}

impl Client {
    pub fn new(base_url: &str, api_key: &str, handle: &Handle, pool_size: u32) -> Result<Client> {
        let addr = Url::parse(base_url)
            .map_err(|e| Error::io(IoErrorKind::InvalidInput, e))?
            .to_socket_addrs()?
            .next()
            .ok_or(Error::io(
                IoErrorKind::AddrNotAvailable,
                "no socket address",
            ))?;

        let pool = ConnPool::builder()
            .max_size(pool_size)
            .build(TcpStreamManager::new(addr))
            .map_err(|e| Error::io(IoErrorKind::Other, e))?;

        Ok(Client {
            base_url: base_url.to_owned(),
            api_key: api_key.to_owned(),
            handle: handle.clone(),
            pool: pool,
        })
    }

    pub fn get<'rq, 'rsp, T, P>(
        &self,
        storage: &'rsp mut String,
        params: P,
    ) -> Response<'rsp, T>
    where
        P: RequestParams + Debug,
        T: LastfmType<'rsp> + Send + 'rsp,
    {
        // TODO: this may fail. need to address in lastfm_parse_rs
        let url: Url = Into::into(Request::new(&self.base_url, &self.api_key, params));
        request!(self, url, get!(url, storage))
    }

    fn get_http_stream(&self) -> Result<TcpStream> {
        let stream = self.pool.get().map_err(
            |e| Error::io(IoErrorKind::Other, e),
        )?;
        TcpStream::from_stream(stream.try_clone()?, &self.handle).map_err(From::from)
    }

    fn get_https_stream(&self, hostname: &str) -> ConnectAsync<TcpStream> {
        let tls = TlsConnector::builder().unwrap().build().unwrap();
        let stream = self.get_http_stream().unwrap();
        tls.connect_async(hostname, stream)
    }
}
