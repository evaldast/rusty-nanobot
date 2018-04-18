use std::error::Error;
use tokio_core::reactor::Core;
use hyper::Client;
use futures::{Future, Stream};
use hyper::{Method, Request};
use hyper::header::{ContentLength, ContentType};
use std::str;

pub fn call_wallet() -> Result<String, Box<Error>> {
    let mut core = Core::new()?;
    let client = Client::new(&core.handle());

    let json = r#"{"action":"block_count"}"#;
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

    return Ok(str::from_utf8(&posted)?.to_string());
}