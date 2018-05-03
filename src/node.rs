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
    pub wallet: String,

    #[serde(default)]
    pub email: String
}

#[derive(Deserialize)]
pub struct Key {
    pub account: String,
    pub public: String,
    pub private: String,
}

#[derive(Deserialize)]
pub struct Balance {
    pub balance: String,
    pub pending: String
}

#[derive(Deserialize)]
pub struct Wallet {
    pub wallet: String
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

#[derive(Serialize)]
struct WalletCommand {
    action: &'static str,
    wallet: String,
    key: String
}

pub fn create_new_key() -> Result<Key, Box<Error>> {
    let json_command: String = serde_json::to_string(&BasicCommand {action: "key_create"})?;

    return Ok(serde_json::from_slice(&call_node(json_command)?).unwrap());
}

pub fn create_new_wallet() -> Result<Wallet, Box<Error>> {
    let json_command: String = serde_json::to_string(&BasicCommand {action: "wallet_create"})?;

    return Ok(serde_json::from_slice(&call_node(json_command)?).unwrap());
}

pub fn get_balance(account: String) -> Result<Balance, Box<Error>> {
    let json_command: String = serde_json::to_string(&AccountCommand {action: "account_balance", account: account})?;
    
    return Ok(serde_json::from_slice(&call_node(json_command)?).unwrap())
}

pub fn add_key_to_wallet(wallet: &str, key: &str) -> Result<(), Box<Error>> {
    let json_command: String = serde_json::to_string(&WalletCommand {action: "wallet_add", wallet: String::from(&*wallet), key: String::from(&*key)})?;

    match call_node(json_command) {
        Ok(_) => Ok(()),
        Err(e) => Err(e)
    }
}

fn call_node(json_command: String) -> Result<Chunk, Box<Error>> {
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