use super::*;

#[test]
fn client_single_request() {
    use lastfm::structs::user::{GetInfo, Params};

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let client = Client::new(
        "http://ws.audioscrobbler.com/2.0/",
        "143f59fafebb6ba4bbfafc6af666e1d6",
        &handle,
        4,
    ).unwrap();

    let mut _me = String::new();
    let info = client.get(&mut _me, Params::GetInfo { user: "xenzh" });
    let res: Result<GetInfo> = core.run(info);

    println!("Response: {:?}", res.unwrap());
}

#[test]
fn client_double_request() {
    use lastfm::structs::user::{GetInfo, Params};

    let mut core = Core::new().unwrap();
    let handle = core.handle();

    let client = Client::new(
        "http://ws.audioscrobbler.com/2.0/",
        "143f59fafebb6ba4bbfafc6af666e1d6",
        &handle,
        4,
    ).unwrap();

    let mut _me = String::new();
    let me = client.get(&mut _me, Params::GetInfo { user: "xenzh" });

    let mut _igor = String::new();
    let igor = client.get(&mut _igor, Params::GetInfo { user: "anmult" });

    let info = me.join(igor);

    let res: Result<(GetInfo, GetInfo)> = core.run(info);

    println!("Response: {:?}", res);
}
