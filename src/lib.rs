#![feature(underscore_lifetimes)]

extern crate url;
extern crate tokio_core;
extern crate tokio_io;
extern crate r2d2;

extern crate lastfm_parse_rs as lastfm;
extern crate async_http_client;


use std::fmt::Debug;
use std::io::{Error as IoError, ErrorKind as IoErrorKind, Result as IoResult};
use std::net::ToSocketAddrs;

use url::Url;

use tokio_core::reactor::Handle;

use r2d2::Pool as ConnPool;

use lastfm::methods::Method;
use lastfm::{LastfmType, Request, RequestParams, from_json_str, Result as LastfmResult};

use async_http_client::prelude::*;
use async_http_client::HttpRequest;

mod pool;

use pool::TcpStreamManager;

// ----------------------------------------------------------------

type BoxedFuture<'de, T> = Box<Future<Item = T, Error = IoError> + Send + 'de>;

pub struct Client {
    base_url: String,
    api_key: String,
    handle: Handle,
    pool: ConnPool<TcpStreamManager>,
}

impl Client {
    pub fn new(base_url: &str, api_key: &str, handle: &Handle, pool_size: u32) -> Result<Client, IoError> {
        let addr = Url::parse(base_url)
            .map_err(|e| IoError::new(IoErrorKind::Other, e))?
            .to_socket_addrs()?
            .next().ok_or(IoError::new(IoErrorKind::Other, "no socket address"))?;

        let pool = ConnPool::builder()
            .max_size(pool_size)
            .build(TcpStreamManager::new(addr))
            .map_err(|e|IoError::new(IoErrorKind::Other, e))?;

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
        method: Method,
        params: P,
    ) -> BoxedFuture<'rsp, LastfmResult<T>>
    where
        T: LastfmType<'rsp> + Send + 'rsp,
        P: RequestParams + Debug,
    {
        let url: Url = Into::into(Request::new(&self.base_url, &self.api_key, method, params));
        let req = HttpRequest::get(url).unwrap();
        let addr = req.addr().unwrap();

        let fut = TcpStream::connect(&addr, &self.handle).and_then(|conn| {
            req.send(conn).and_then(move |res| {
                if let (Some(resp), _) = res {
                    *storage = String::from_utf8_lossy(resp.get_body()).into_owned();
                    let data: LastfmResult<T> = from_json_str(storage);
                    return ok(data);
                }
                err(IoError::new(IoErrorKind::Other, "for now"))
            })
        });

        Box::new(fut)
    }

    fn get_stream(&self) -> IoResult<TcpStream> {
        let stream = self.pool.get().map_err(|e| IoError::new(IoErrorKind::Other, e))?;
        TcpStream::from_stream(stream.try_clone()?, &self.handle)
    }
}

// ----------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use lastfm::structs::user::{GetInfo, Params};

    #[test]
    fn client_test_run() {
        let mut core = Core::new().unwrap();
        let handle = core.handle();

        let client = Client::new(
            "http://ws.audioscrobbler.com/2.0/",
            "143f59fafebb6ba4bbfafc6af666e1d6",
            &handle,
            4,
        ).unwrap();

        let mut storage = String::new();

        let fut: BoxedFuture<'_, LastfmResult<GetInfo>> = client.get(
            &mut storage,
            Method::UserGetInfo,
            Params::GetInfo { user: "xenzh" },
        );
        let res = core.run(fut).unwrap();

        println!("Response: {:?}", res.unwrap());
    }
}
