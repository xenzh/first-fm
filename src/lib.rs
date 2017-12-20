extern crate url;
extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate native_tls;
extern crate tokio_tls;

extern crate lastfm_parse_rs as lastfm;
extern crate async_http_client;

// ----------------------------------------------------------------

/// Contains return types such as errors, results and futures
pub mod utils;

/// Contains API client and builder structures
pub mod client;

#[cfg(test)]
mod tests;

// ----------------------------------------------------------------

/// Base URL for last.fm API methods. Client uses it as a default.
/// Note that majority of methods also work on bare HTTP.
pub static LASTFM_API_BASE_URL: &str = "https://ws.audioscrobbler.com/2.0/";

/// Base URL for last.fm authentication API for desktop applications.
/// Client uses it as a default.
pub static LASTFM_API_AUTH_URL: &str = "https://www.last.fm/api/auth/";

// ----------------------------------------------------------------

pub use utils::{Error, Result, Data};
pub use client::{Client, Builder};
