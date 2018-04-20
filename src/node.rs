use std::error::Error;
use tokio_core::reactor::Core;
use hyper::Client;
use futures::{Future, Stream};
use hyper::{Method, Request, Chunk};
use hyper::header::{ContentLength, ContentType};
use std::str;
use serde_json;

#[derive(Deserialize)]
pub struct NewAccount {
    pub account: String,
    pub public: String,
    pub private: String,

    #[serde(default)]
    pub email: String
}

pub fn create_new_account() -> Result<NewAccount, Box<Error>> {
    return Ok(serde_json::from_slice(&call_wallet(r#"{"action":"key_create"}"#)?).unwrap());
}

fn call_wallet(json_command: &'static str) -> Result<Chunk, Box<Error>> {
    let mut core = Core::new()?;
    let client = Client::new(&core.handle());
    let uri = "http://127.0.0.1:7076".parse()?;
    let mut req = Request::new(Method::Post, uri);

    req.headers_mut().set(ContentType::json());
    req.headers_mut().set(ContentLength(json_command.len() as u64));
    req.set_body(json_command);

    let post = client.request(req).and_then(|res| {
        res.body().concat2()
    });

    let response = core.run(post).unwrap();

    return Ok(response);
}