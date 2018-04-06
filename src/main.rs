#![feature(plugin, decl_macro)]
#![plugin(rocket_codegen)]

extern crate rocket;

extern crate rocket_contrib;

#[macro_use]
extern crate serde_derive;

use rocket_contrib::Json;

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

    match event.0.event_type.trim() {
        "ADDED_TO_SPACE" => {
            return Json(ResponseMessage {
                text: "Hello and thanks for adding me, ".to_string() + &event.0.user.display_name,
            })
        }
        "MESSAGE" => {
            return Json(ResponseMessage {
                text: "Thanks for messaging me, ".to_string() + &event.0.user.display_name,
            })
        }
        _ => {
            return Json(ResponseMessage {
                text: "Sorry, did not understand that".to_string(),
            })
        }
    };
}

#[get("/")]
fn moo() -> &'static str {
    "Mooo, from Uboontoo!"
}

fn main() {
    rocket::ignite()
        .mount("/", routes![post_json, moo])
        .launch();
}
