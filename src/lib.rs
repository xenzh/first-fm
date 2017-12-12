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
mod builder;
mod client;

#[cfg(test)]
mod tests;

// ----------------------------------------------------------------

pub use utils::{Error, Result, Response};
pub use builder::Builder;
pub use client::Client;
