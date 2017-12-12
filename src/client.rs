use std::fmt::Debug;
use std::io::ErrorKind as IoErrorKind;
use std::net::ToSocketAddrs;

use url::Url;

use r2d2::Pool as ConnPool;

use tokio_core::reactor::{Core, Handle};

use native_tls::TlsConnector;
use tokio_tls::TlsConnectorExt;

use lastfm::{LastfmType, Request, RequestParams, from_json_str};
use lastfm::auth::{Params as AuthParams, GetMobileSession};

use async_http_client::prelude::*;
use async_http_client::HttpRequest;

// ----------------------------------------------------------------

use pool::TcpStreamManager;
use utils::{Error, Result, Response};

// ----------------------------------------------------------------

macro_rules! request {
    ($client:ident, $url:expr, $method:expr) => {
        match $url.scheme() {
            "http" => Box::new(result($client.get_stream()).and_then($method)),
            "https" => {
                let tls = TlsConnector::builder().unwrap().build().unwrap();
                Box::new(result($client.get_stream()).and_then(move |stream| {
                    tls.connect_async($url.domain().unwrap(), stream)
                        .map_err(From::from)
                        .and_then($method)
                }))
            }
            _ => Box::new(err(Error::io(IoErrorKind::InvalidInput, "no scheme in url"))),
        }
    }
}

macro_rules! get {
    ($url:expr, $buf:ident) => {
        |stream| {
            result(HttpRequest::get($url).map_err(|e| Error::io(IoErrorKind::Other, e)))
                .and_then(send!(stream, $buf))
        }
    }
}

macro_rules! post {
    ($base_url:ident, $url:ident, $buf:ident) => {
        move |stream| {
            let query = $url.query().unwrap_or("");
            result(HttpRequest::post($base_url, query).map_err(|e| Error::io(IoErrorKind::Other, e)))
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

#[derive(Debug)]
pub struct Client {
    base_url: Url,
    api_key: String,
    secret: Option<String>,
    session: Option<String>,
    handle: Handle,
    pool: ConnPool<TcpStreamManager>,
}

impl Client {
    pub fn new(
        base_url: &str,
        api_key: &str,
        secret: Option<&str>,
        handle: &Handle,
        pool_size: u32
    ) -> Result<Client> {
        let url = Url::parse(base_url)
            .map_err(|e| Error::io(IoErrorKind::InvalidInput, e))?;

        let addr = url
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
            base_url: url,
            api_key: api_key.to_owned(),
            secret: secret.map(|s| s.to_owned()),
            session: None,
            handle: handle.clone(),
            pool: pool,
        })
    }

    pub fn request<'rq, 'rsp, T, P>(
        &self,
        storage: &'rsp mut String,
        params: P,
    ) -> Response<'rsp, T>
    where
        P: RequestParams + Debug,
        T: LastfmType<'rsp> + Send + 'rsp,
    {
        let secret = self.secret.as_ref().map(|s| s.as_str());
        let is_write = params.is_write();

        match Request::new(self.base_url.as_str(), &self.api_key, secret, params).get_url() {
            Ok(url) => {
                if is_write {
                    let base_url = self.base_url.clone();
                    request!(self, url, post!(base_url, url, storage))
                } else {
                    request!(self, url, get!(url, storage))            
                }
            }
            Err(e) => Box::new(err(From::from(e))),
        }
    }

    pub fn auth(&mut self, core: &mut Core, username: &str, password: &str) -> Result<()> {
        let mut _buf = String::new();
        let auth = self.request(
            &mut _buf,
            AuthParams::GetMobileSession { username: username, password: password }
        );
        let resp: GetMobileSession = core.run(auth)?;
        self.session = Some(resp.key.to_owned());
        Ok(())
    }

    pub fn is_authenticated(&self) -> bool {
        self.session.is_some()
    }

    pub fn set_session_key(&mut self, key: &str) {
        self.session = Some(key.to_owned());
    }

    fn get_stream(&self) -> Result<TcpStream> {
        let stream = self.pool.get().map_err(
            |e| Error::io(IoErrorKind::Other, e),
        )?;
        TcpStream::from_stream(stream.try_clone()?, &self.handle).map_err(From::from)
    }
}
