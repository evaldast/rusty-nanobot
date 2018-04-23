use std::error::Error;
use tokio_core::reactor::Core;
use hyper::Client;
use futures::{Future, Stream};
use hyper::{Method, Request, Chunk};
use hyper::header::{ContentLength, ContentType};
use std::str;
use serde_json;

#[derive(Deserialize)]
pub struct Account {
    pub account: String,
    pub public: String,
    pub private: String,

    #[serde(default)]
    pub email: String
}

#[derive(Deserialize)]
pub struct Balance {
    pub balance: u64,
    pub pending: u64
}

#[derive(Serialize)]
struct BasicCommand {
    action: &'static str
}

#[derive(Serialize)]
struct AccountCommand {
    action: &'static str,
    account: String
}

pub fn create_new_account() -> Result<Account, Box<Error>> {
    let json_command:String = serde_json::to_string(&BasicCommand {action: "key_create"})?;

    return Ok(serde_json::from_slice(&call_wallet(json_command)?).unwrap());
}

pub fn get_balance(account: String) -> Result<Balance, Box<Error>> {
    let json_command:String = serde_json::to_string(&AccountCommand {action: "account_balance", account: account})?;
    
    return Ok(serde_json::from_slice(&call_wallet(json_command)?).unwrap())
}

fn call_wallet(json_command: String) -> Result<Chunk, Box<Error>> {
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