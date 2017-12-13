extern crate open;

use super::*;
use futures::future::Future;
use tokio_core::reactor::Core;

// ----------------------------------------------------------------

static LASTFM_BASE_URL_HTTP: &str = "http://ws.audioscrobbler.com/2.0/";
static LASTFM_API_KEY: &str = "api_key";
static LASTFM_API_SECRET: &str = "secret";
static LASTFM_USERNAME: &str = "username";
static LASTFM_PASSWORD: &str = "password";

// ----------------------------------------------------------------

#[test]
fn single_http() {
    use lastfm::user::{GetInfo, Params};

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let client = Client::builder()
        .base_url(LASTFM_BASE_URL_HTTP)
        .api_key(LASTFM_API_KEY)
        .handle(handle.clone())
        .build()
        .unwrap();

    let mut _me = String::new();
    let info = client.request(&mut _me, Params::GetInfo { user: "xenzh" });
    let res: Result<GetInfo> = core.run(info);

    println!("Response: {:?}", res);
    assert!(res.is_ok());
}

#[test]
fn double_https() {
    use lastfm::user::{GetInfo, Params};

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let client = Client::builder()
        .api_key(LASTFM_API_KEY)
        .handle(handle.clone())
        .build()
        .unwrap();

    let mut _me = String::new();
    let me = client.request(&mut _me, Params::GetInfo { user: "xenzh" });

    let mut _igor = String::new();
    let igor = client.request(&mut _igor, Params::GetInfo { user: "anmult" });

    let info = me.join(igor);

    let res: Result<(GetInfo, GetInfo)> = core.run(info);

    println!("Response: {:?}", res);
    assert!(res.is_ok());
}

#[test]
fn post_https() {
    use lastfm::auth::{Params, GetToken};

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let client = Client::builder()
        .api_key(LASTFM_API_KEY)
        .secret(LASTFM_API_SECRET)
        .handle(handle.clone())
        .build()
        .unwrap();

    let mut _me = String::new();
    let token = client.request(&mut _me, Params::GetToken);
    let res: Result<GetToken> = core.run(token);

    println!("Response: {:?}", res);
    assert!(res.is_ok());
}

#[test]
fn mobile_auth() {
    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let mut client = Client::builder()
        .api_key(LASTFM_API_KEY)
        .secret(LASTFM_API_SECRET)
        .handle(handle.clone())
        .build()
        .unwrap();

    assert!(!client.is_authenticated());

    let res = client.mobile_auth(&mut core, LASTFM_USERNAME, LASTFM_PASSWORD);

    println!("Response: {:?}\nIs authenticated? {}\n",
        res,
        client.is_authenticated(),
    );
    assert!(client.is_authenticated());
}

#[test]
fn desktop_auth() {
    use std::{thread, time};

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let mut client = Client::builder()
        .api_key(LASTFM_API_KEY)
        .secret(LASTFM_API_SECRET)
        .handle(handle.clone())
        .build()
        .unwrap();

    assert!(client.finalize_desktop_auth(&mut core).is_err());
    assert!(!client.is_authenticated());

    let auth_url = client.init_desktop_auth(&mut core).unwrap();

    let res = open::that(auth_url.as_str());
    println!("url open result: {:?}", res);
    assert!(res.is_ok());

    thread::sleep(time::Duration::from_secs(15));

    let res = client.finalize_desktop_auth(&mut core);
    println!("finalize_desktop_auth() result: {:?}", res);
    println!(r#"
        This test opens lastfm API permissions link in the browser.
        In order for the test to pass, you have to click "Allow access"
        on opened page.
    "#);
    assert!(res.is_ok());

    assert!(client.is_authenticated());
}