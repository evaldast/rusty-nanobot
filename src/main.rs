#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
extern crate futures;
extern crate hyper;
extern crate tokio_core;

#[macro_use]
extern crate serde_derive;

use rocket_contrib::Json;
use hyper::{Method, Request};
use hyper::header::{ContentLength, ContentType};
use hyper::Client;
use futures::{Future, Stream};
use tokio_core::reactor::Core;
use std::str;
use std::error::Error;

#[derive(Deserialize, Debug)]
struct Event {
    #[serde(rename = "configCompleteRedirectUrl", default)]
    config_complete_redirect_url: String,

    #[serde(rename = "eventTime")]
    event_time: String,

    #[serde(default)]
    message: Message,

    space: Space,
    token: String,

    #[serde(rename = "type")]
    event_type: String,

    user: Sender,
}

#[derive(Deserialize, Debug, Default)]
struct Space {
    name: String,

    #[serde(rename = "type")]
    message_type: String,
}

#[derive(Deserialize, Debug, Default)]
struct Sender {
    #[serde(rename = "avatarUrl")]
    avatar_url: String,

    #[serde(rename = "displayName")]
    display_name: String,

    email: String,
    name: String,

    #[serde(rename = "type")]
    sender_type: String,
}

#[derive(Deserialize, Debug, Default)]
struct Thread {
    name: String,

    #[serde(rename = "retentionSettings")]
    retention_settings: RetentionSettings,
}

#[derive(Deserialize, Debug, Default)]
struct RetentionSettings {
    state: String,
}

#[derive(Deserialize, Debug, Default)]
struct Message {
    #[serde(rename = "createTime")]
    create_time: String,

    name: String,
    sender: Sender,
    space: Space,
    text: String,
    thread: Thread,
}

#[derive(Serialize)]
struct ResponseMessage {
    text: String,
}

#[post("/hello", format = "application/json", data = "<event>")]
fn post_json(event: Json<Event>) -> Json<ResponseMessage> {
    println!("{:?}", &event.0);

    call_wallet();

    match event.0.event_type.trim() {
        "ADDED_TO_SPACE" => {
            return Json(ResponseMessage {
                text: format!("Hello and thanks for adding me, *{}*. For help type `!help`", event.0.user.display_name),
            })
        }
        "MESSAGE" => {
            return Json(ResponseMessage {
                text: parse_text(event.0.message.text, event.0.user.display_name)
            })
        }
        _ => {
            return Json(ResponseMessage {
                text: "Unsupported event".to_string(),
            })
        }
    };
}

#[get("/")]
fn moo() -> &'static str {
    "Mooo, from Uboontoo!"
}

fn call_wallet() -> Result<(), Box<Error>> {
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

    println!("POST: {}", str::from_utf8(&posted)?);

    return Ok(());
}

fn main() {
    rocket::ignite()
        .mount("/", routes![post_json, moo])
        .launch();
}

fn parse_text(text: String, display_name: String) -> String {
    return match text.trim() {
        "!help" => "Not implemented yet".to_string(),
        _ => format!("Did not quite catch that, *{}*, type `!help` for help", display_name)
    }
}
