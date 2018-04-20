use std::error::Error;
use tokio_core::reactor::Core;
use hyper::Client;
use futures::{Future, Stream};
use hyper::{Method, Request};
use hyper::header::{ContentLength, ContentType};
use std::str;
use serde_json::{from_str, from_slice};

#[derive(Serialize, Deserialize)]
pub struct NewAccount {
    pub account: String,
    public: String,
    private: String,
    email: String
}

pub fn call_wallet() -> Result<NewAccount, Box<Error>> {
    let mut core = Core::new()?;
    let client = Client::new(&core.handle());

    let json = r#"{"action":"key_create"}"#;
    let uri = "http://127.0.0.1:7076".parse()?;
    let mut req = Request::new(Method::Post, uri);
    req.headers_mut().set(ContentType::json());
    req.headers_mut().set(ContentLength(json.len() as u64));
    req.set_body(json);

    let post = client.request(req).and_then(|res| {
        println!("POST: {}", res.status());

        res.body().concat2()
    });

    let posted = core.run(post).unwrap();
    let acc:NewAccount = from_slice(&posted).unwrap();
    //let account:NewAccount = from_str(str::from_utf8(&posted)?)?;

    return Ok(acc);
}