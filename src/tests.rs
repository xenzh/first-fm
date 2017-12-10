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

    let mut storage = String::new();

    let fut = client.get(
        &mut storage,
        Method::UserGetInfo,
        Params::GetInfo { user: "xenzh" },
    );

    let res: Result<GetInfo> = core.run(fut);

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

    let mut sme = String::new();
    let me = client.get(
        &mut sme,
        Method::UserGetInfo,
        Params::GetInfo { user: "xenzh" },
    );

    let mut sigor = String::new();
    let igor = client.get(
        &mut sigor,
        Method::UserGetInfo,
        Params::GetInfo { user: "anmult" },
    );

    let info = me.join(igor);
    let res: Result<(GetInfo, GetInfo)> = core.run(info);

    println!("Response: {:?}", res);
}
