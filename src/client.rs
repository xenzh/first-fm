use std::fmt::Debug;
use std::io::ErrorKind as IoErrorKind;
use std::net::{ToSocketAddrs, SocketAddr};

use url::Url;

use tokio_core::reactor::{Core, Handle};
use tokio_core::net::TcpStreamNew;

use native_tls::TlsConnector;
use tokio_tls::TlsConnectorExt;

use lastfm::{LastfmType, Request, RequestParams, from_json_str};
use lastfm::auth::{Params as AuthParams, GetMobileSession, GetToken, GetSession};

use async_http_client::prelude::*;
use async_http_client::HttpRequest;

// ----------------------------------------------------------------

use super::{LASTFM_API_BASE_URL, LASTFM_API_AUTH_URL};
use utils::{Error, Result, Data};

// ----------------------------------------------------------------

/// Client builder
///
/// Base and desktop auth urls are automatically set to defaults.
///
/// To make `read` calls API key and Tokio reactor core handle have to be set.
///
/// To make `auth` and `write` calls secret has to be set.
pub struct Builder {
    base_url: String,
    auth_url: String,
    api_key: Option<String>,
    secret: Option<String>,
    handle: Option<Handle>,
}

impl Builder {
    /// Constructs new client builder
    pub fn new() -> Builder {
        Builder {
            base_url: LASTFM_API_BASE_URL.to_owned(),
            auth_url: LASTFM_API_AUTH_URL.to_owned(),
            api_key: None,
            secret: None,
            handle: None,
        }
    }

    /// Builds new client from builder configuration
    pub fn build(self) -> Result<Client> {
        let base_url: Url = self.base_url.parse().map_err(|e| Error::build(e))?;
        let auth_url: Url = self.auth_url.parse().map_err(|e| Error::build(e))?;

        let api_key = self.api_key.ok_or(Error::build("Missing API key"))?;
        let handle = self.handle.ok_or(
            Error::build("Missing Tokio reactor core handle"),
        )?;

        let addr = base_url.to_socket_addrs()?.next().ok_or(Error::build(
            "No socket address found in base url",
        ))?;

        Ok(Client {
            base_url: base_url,
            auth_url: auth_url,
            socket_addr: addr,
            api_key: api_key,
            secret: self.secret,
            session: None,
            token: None,
            handle: handle,
        })
    }

    /// Updates base API url
    pub fn base_url(mut self, url: &str) -> Builder {
        self.base_url = url.to_owned();
        self
    }

    /// Updates base desktop auth url
    pub fn auth_url(mut self, url: &str) -> Builder {
        self.auth_url = url.to_owned();
        self
    }

    /// Sets API key
    pub fn api_key(mut self, api_key: &str) -> Builder {
        self.api_key = Some(api_key.to_owned());
        self
    }

    /// Sets Tokio reactor core handle
    pub fn handle(mut self, handle: Handle) -> Builder {
        self.handle = Some(handle);
        self
    }

    /// Sets API shared secret
    pub fn secret(mut self, secret: &str) -> Builder {
        self.secret = Some(secret.to_owned());
        self
    }
}

// ----------------------------------------------------------------

macro_rules! request {
    ($client:ident, $url:expr, $method:expr) => {
        match $url.scheme() {
            "http" => {
                Box::new($client.get_stream().map_err(From::from).and_then($method))
            },
            "https" => {
                let tls = TlsConnector::builder().unwrap().build().unwrap();
                Box::new($client.get_stream().map_err(From::from).and_then(move |stream| {
                    tls.connect_async($url.domain().unwrap(), stream)
                        .map_err(From::from)
                        .and_then($method)
                }))
            },
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
    ($stream:expr, $buf:ident) => {
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

/// last.fm API client
/// TODO: write something useful about low-level `request` and
/// high-level `auth` and `scrobble` APIs
pub struct Client {
    base_url: Url,
    auth_url: Url,
    socket_addr: SocketAddr,
    api_key: String,
    secret: Option<String>,
    token: Option<String>,
    session: Option<String>,
    handle: Handle,
}

impl Client {
    /// Returns new client builder
    pub fn builder() -> Builder {
        Builder::new()
    }

    /// Main entry point of low-level `request` client API.
    ///
    /// Fetches last.fm data objects based on given request parameters.
    /// Parameters type has to implement `RequestParams` trait and data type
    /// has to implement `LastfmType` - both these traits and itheir implementations
    /// can be found in [lastfm-parse-rs crate](https://xenzh.github.io/lastfm-parse-rs/).
    ///
    /// This method returns a future that won't ever resolve unless consumed by event loop.
    ///
    /// Note that this method somewhat awkwardly requires a mutable string to be supplied.
    /// Reason for this is that all `lastfm_parse_rs` types heavily rely on so-called zero-cost
    /// deserialization recently introduced in serde. The idea is that instead of copying, string
    /// field values are borrowed directly from raw response body.
    /// This sounds like a great performance saving at cost of some convenience: response body
    /// must live as long as parsed object. And that's what this mutable string is here to store.
    ///
    /// ## Example:
    /// ```
    /// use tokio_core::reactor::Core;
    /// use lastfm_parse_rs::user::{Params, GetInfo};
    /// use first_fm::{Client, Result};
    ///
    /// let mut core = Core::new().unwrap();
    /// let handle = core.handle();
    ///
    /// let client = Client::builder()
    ///     .api_key(LASTFM_API_KEY)
    ///     .handle(handle.clone())
    ///     .build()
    ///     .unwrap();
    ///
    /// let mut _buf = String::new();
    /// let me = client.request(&mut _buf, Params::GetInfo { user: "xenzh" });
    /// let res: Result<GetInfo> = core.run(info);
    ///
    /// println!("Result: {:?}", res);
    /// ```
    pub fn request<'rq, 'rsp, T, P>(
        &self,
        storage: &'rsp mut String,
        params: P,
    ) -> Data<'rsp, T>
    where
        P: RequestParams + Debug,
        T: LastfmType<'rsp> + Send + 'rsp,
    {
        let is_post = params.needs_signature();
        let secret = self.secret.as_ref().map(|s| s.as_str());
        let session = self.session.as_ref().map(|s| s.as_str());

        let rq = Request::new(self.base_url.as_str(), &self.api_key, secret, session, params);
        match rq.get_url() {
            Ok(url) => {
                if is_post {
                    let base_url = self.base_url.clone();
                    request!(self, url, post!(base_url, url, storage))
                } else {
                    request!(self, url, get!(url, storage))
                }
            }
            Err(e) => Box::new(err(From::from(e))),
        }
    }

    /// This is a `high-level` client API method for one of the possible ways to authenticate.
    ///
    /// According to API documentation this path is intended to be used in standalone mobile apps,
    /// but works fine from elsewhere (and is the easiest).
    ///
    /// Client is authenticated with given user credentials.
    /// Once this method succeeds, client will be able to successfully call `write` API methods.
    ///
    /// Check https://www.last.fm/api/mobileauth for details.
    pub fn mobile_auth(&mut self, core: &mut Core, username: &str, password: &str) -> Result<()> {
        let mut _buf = String::new();
        let auth = self.request(
            &mut _buf,
            AuthParams::GetMobileSession {
                username: username,
                password: password,
            },
        );
        let resp: GetMobileSession = core.run(auth)?;
        self.session = Some(resp.key.to_owned());
        Ok(())
    }

    /// First phase of desktop auth path.
    ///
    /// It returns an url, that should be opened in user's browser.
    /// User then should accept application request by clicking a button in 1 hour.
    /// After that `finalize_desktop_auth()` should be called to complete auth process.
    ///
    /// Check https://www.last.fm/api/desktopauth and `finalize_desktop_auth()` for details.
    pub fn init_desktop_auth(&mut self, core: &mut Core) -> Result<Url> {
        let mut _buf = String::new();
        let get_token = self.request(&mut _buf, AuthParams::GetToken);
        let resp: GetToken = core.run(get_token)?;

        self.token = Some(resp.token.to_owned());

        let mut url = self.auth_url.clone();
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("api_key", &self.api_key);
            query.append_pair("token", resp.token);
        }

        Ok(url)
    }

    /// Second phase of desktop auth path.
    ///
    /// This method should be called after `init_desktop_auth()` once user
    /// allowed access to his profile on the page opened using first phase url.
    ///
    /// Once this method succeeds, client will be able to successfully call `write` API methods.
    ///
    /// Check https://www.last.fm/api/desktopauth and `init_desktop_auth()` for details.
    pub fn finalize_desktop_auth(&mut self, core: &mut Core) -> Result<()> {
        let mut _buf = String::new();
        let token = self.token.take().ok_or(Error::io(
            IoErrorKind::NotFound,
            "Desktop session was not initiated (no auth token found)"
        ))?;

        let get_session = self.request(&mut _buf, AuthParams::GetSession { token: &token });
        let resp: GetSession = core.run(get_session)?;
        self.session = Some(resp.key.to_owned());

        Ok(())
    }

    /// Checks if the client is authenticated and therefore able to call `write` API methods
    pub fn is_authenticated(&self) -> bool {
        self.session.is_some()
    }

    fn get_stream(&self) -> TcpStreamNew {
        TcpStream::connect(&self.socket_addr, &self.handle)
    }
}
