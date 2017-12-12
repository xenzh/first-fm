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

use super::LASTFM_API_BASE_URL;

use pool::TcpStreamManager;
use utils::{Error, Result, Response};

// ----------------------------------------------------------------

pub struct Builder {
    base_url: String,
    connections: u32,
    api_key: Option<String>,
    secret: Option<String>,
    handle: Option<Handle>,
}

impl Builder {
    pub fn new() -> Builder {
        Builder {
            base_url: LASTFM_API_BASE_URL.to_owned(),
            connections: 2,
            api_key: None,
            secret: None,
            handle: None,
        }
    }

    pub fn build(self) -> Result<Client> {
        let base_url: Url = self.base_url.parse().map_err(|e| Error::build(e))?;
        if self.connections < 1 {
            return Err(Error::build("Need to have at least 1 connection to operate"));
        }
        let api_key = self.api_key.ok_or(Error::build("Missing API key"))?;
        let handle = self.handle.ok_or(Error::build("Missing Tokio reactor core handle"))?;

        let addr = base_url
            .to_socket_addrs()?
            .next()
            .ok_or(Error::build("No socket address found in base url"))?;

        let pool = ConnPool::builder()
            .max_size(self.connections)
            .build(TcpStreamManager::new(addr))
            .map_err(|e| Error::build(e))?;

        Ok(Client {
            base_url: base_url,
            api_key: api_key,
            secret: self.secret,
            session: None,
            handle: handle,
            pool: pool,
        })
    }

    pub fn base_url(mut self, url: &str) -> Builder {
        self.base_url = url.to_owned();
        self
    }

    pub fn connections(mut self, connection_count: u32) -> Builder {
        self.connections = connection_count;
        self
    }

    pub fn api_key(mut self, api_key: &str) -> Builder {
        self.api_key = Some(api_key.to_owned());
        self
    }

    pub fn handle(mut self, handle: Handle) -> Builder {
        self.handle = Some(handle);
        self
    }

    pub fn secret(mut self, secret: &str) -> Builder {
        self.secret = Some(secret.to_owned());
        self
    }
}

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

pub struct Client {
    base_url: Url,
    api_key: String,
    secret: Option<String>,
    session: Option<String>,
    handle: Handle,
    pool: ConnPool<TcpStreamManager>,
}

impl Client {
    pub fn builder() -> Builder {
        Builder::new()
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

    fn get_stream(&self) -> Result<TcpStream> {
        let stream = self.pool.get().map_err(
            |e| Error::io(IoErrorKind::Other, e),
        )?;
        TcpStream::from_stream(stream.try_clone()?, &self.handle).map_err(From::from)
    }
}
