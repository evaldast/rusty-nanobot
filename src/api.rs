use db;
use node;
use rocket::Rocket;
use rocket::State;
use rocket_contrib::Json;
use rusqlite::Connection;
use std::sync::Mutex;
use node::{Account, Balance};

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
    retention_settings: RetentionSettings
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
    cards: Option<Vec<Card>>
}

#[derive(Serialize)]
struct Card {
    sections: Vec<Section>
}

#[derive(Serialize)]
struct Section {
    widgets: Vec<Widget>
}

#[derive(Serialize)]
struct Widget {
    image: Image
}

#[derive(Serialize)]
struct Image {
    #[serde(rename = "imageUrl")]
    image_url: String
}

#[post("/hello", format = "application/json", data = "<event>")]
fn post_json(db_conn: State<Mutex<Connection>>, event: Json<Event>) -> Json<ResponseMessage> {
    // println!("{:?}", &event.0);

    match event.0.event_type.trim() {
        "ADDED_TO_SPACE" => {
            return Json(ResponseMessage {
                text: format!("Hello and thanks for adding me, *{}*. For help type `!help`", event.0.user.display_name),
                cards: None
            })
        }
        "MESSAGE" => { 
            return Json(parse_text(event.0.message.text, event.0.user, &db_conn))
            }
        _ => {
            return Json(ResponseMessage {
                text: "Unsupported event".to_string(),
                cards: None
            })
        }
    };
}

#[get("/")]
fn moo() -> String {
    return format!("Wassabi");
}

// fn processMessage(text: String) -> ResponseMessage {

// }

fn parse_text(text: String, user: Sender, db_conn: &Mutex<Connection>) -> ResponseMessage {
    return match remove_bot_name_from_text(text).trim() {
        "!help" => ResponseMessage { text: "Available commands: `!help` `!create_account` `!balance`".to_string(), cards: None },
        "!create_account" => match node::create_new_account() {
            Ok(acc) => ResponseMessage { text: add_account_to_database(acc, user.email, db_conn), cards: None },
            Err(err) => ResponseMessage { text: format!("{}", err), cards: None }
        },
        "!balance" => ResponseMessage { text: get_balance(user.email, db_conn), cards: None },
        _ => ResponseMessage { text: format!("Did not quite catch that, *{}*, type `!help` for help", user.display_name), cards: None }
    };
}

fn remove_bot_name_from_text(text: String) -> String {
    match text.starts_with("@") {
        true => return text.split("@Rusty Nanobot").nth(1).unwrap().to_string(),
        false => return text,
    }
}

fn add_account_to_database(acc: Account, email: String, db_conn: &Mutex<Connection>) -> String {
    match db::add_account(&db_conn, acc, email) {
        Ok(_) => format!("Account has been succesfully created, to check your balance type `!balance`"),
        Err(err) => format!("{}", err)
    }
}

fn get_balance(email: String, db_conn: &Mutex<Connection>) -> String {
    let acc:Account = match db::get_account(db_conn, email) {
        Ok(a) => a,
        Err(err) => return format!("{}", err)
    };

    let bal:Balance = match node::get_balance(acc.account) {
        Ok(b) => b,
        Err(err) => return format!("{}", err)
    };

    return format!("Current balance: {}; Pending: {}", bal.balance, bal.pending);
}

pub fn rocket(db_conn: Mutex<Connection>) -> Rocket {
    Rocket::ignite()
        .manage(db_conn)
        .mount("/", routes![post_json, moo])
}
