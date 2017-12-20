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

#[test]
fn write_album_add_tags() {
    use lastfm::album::{Params, AddTags};

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let mut client = Client::builder()
        .api_key(LASTFM_API_KEY)
        .secret(LASTFM_API_SECRET)
        .handle(handle.clone())
        .build()
        .unwrap();

    assert!(client.mobile_auth(&mut core, LASTFM_USERNAME, LASTFM_PASSWORD).is_ok());

    let mut _buf = String::new();
    let add_tags = client.request(&mut _buf, Params::AddTags {
        artist: "iamthemorning",
        album: "~",
        tags: "acoustic, chamber pop, progressive rock, female vocalists",
    });

    let resp: Result<AddTags> = core.run(add_tags);
    println!("Response: {:?}", resp);
    assert!(resp.is_ok());
}

#[test]
#[allow(non_snake_case)]
fn write_track_update_now_playing() {
    use lastfm::track::{Params, UpdateNowPlaying};

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let mut client = Client::builder()
        .api_key(LASTFM_API_KEY)
        .secret(LASTFM_API_SECRET)
        .handle(handle.clone())
        .build()
        .unwrap();

    assert!(client.mobile_auth(&mut core, LASTFM_USERNAME, LASTFM_PASSWORD).is_ok());

    let mut _buf = String::new();
    let nowplaying = client.request(&mut _buf, Params::UpdateNowPlaying {
        artist: "iamthemorning",
        track: "touching ii",
        album: Some("~"),
        trackNumber: Some(9),
        context : None,
        mbid: None,
        duration: Some(244),
        albumArtist : None,
    });

    let resp: Result<UpdateNowPlaying> = core.run(nowplaying);
    println!("Response: {:?}", resp);
    assert!(resp.is_ok());
}

#[test]
fn write_track_scrobble_raw() {
    use lastfm::track::{Params, ScrobbleTrack, Scrobble};

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let mut client = Client::builder()
        .api_key(LASTFM_API_KEY)
        .secret(LASTFM_API_SECRET)
        .handle(handle.clone())
        .build()
        .unwrap();

    assert!(client.mobile_auth(&mut core, LASTFM_USERNAME, LASTFM_PASSWORD).is_ok());

    // Single
    let mut _single = String::new();
    let single = vec!(ScrobbleTrack::new("bloody woods".to_string(), "intro".to_string(), 1513719209));
    let scrobble_single = client.request(&mut _single, Params::Scrobble { batch: &single });

    let resp: Result<Scrobble> = core.run(scrobble_single);
    println!("\nResponse (single): {:?}\n", resp);
    assert!(resp.is_ok());

    // Batch
    let mut _batch = String::new();
    let batch = vec!(
        ScrobbleTrack::new("iamthemorning".to_string(), "touching ii".to_string(), 1513719309),
        ScrobbleTrack::new("schtimm".to_string(), "sunotic drive".to_string(), 1513719399)
    );
    let scrobble_batch = client.request(&mut _batch, Params::Scrobble { batch: &batch });

    let resp: Result<Scrobble> = core.run(scrobble_batch);
    println!("Response: {:?}", resp);
    assert!(resp.is_ok());
}