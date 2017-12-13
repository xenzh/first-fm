extern crate url;
extern crate r2d2;
extern crate futures;
extern crate tokio_core;
extern crate native_tls;
extern crate tokio_tls;

extern crate lastfm_parse_rs as lastfm;
extern crate async_http_client;

// ----------------------------------------------------------------

mod pool;
mod utils;
mod client;

#[cfg(test)]
mod tests;

// ----------------------------------------------------------------

pub static LASTFM_API_BASE_URL: &str = "https://ws.audioscrobbler.com/2.0/";
pub static LASTFM_API_AUTH_URL: &str = "https://www.last.fm/api/auth/";

pub use utils::{Error, Result, Data};
pub use client::{Client, Builder};
