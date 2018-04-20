use db;
use node;
use rocket::Rocket;
use rocket::State;
use rocket_contrib::Json;
use rusqlite::Connection;
use std::sync::Mutex;

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
}

#[post("/hello", format = "application/json", data = "<event>")]
fn post_json(db_conn: State<Mutex<Connection>>, event: Json<Event>) -> Json<ResponseMessage> {
    // println!("{:?}", &event.0);

    match event.0.event_type.trim() {
        "ADDED_TO_SPACE" => {
            return Json(ResponseMessage {
                text: format!(
                    "Hello and thanks for adding me, *{}*. For help type `!help`",
                    event.0.user.display_name
                ),
            })
        }
        "MESSAGE" => {
            return Json(ResponseMessage {
                text: parse_text(event.0.message.text, event.0.user, &db_conn),
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
fn moo() -> String {
    return format!("Wassabi");
}

fn parse_text(text: String, user: Sender, db_conn: &Mutex<Connection>) -> String {
    return match remove_bot_name_from_text(text).trim() {
        "!help" => "Available commands: `!help` `!create_account`".to_string(),
        "!create_account" => match node::create_new_account() {
            Ok(acc) => add_account_to_database(acc, user.email, db_conn),
            Err(err) => format!("{}", err),
        },
        _ => format!("Did not quite catch that, *{}*, type `!help` for help", user.display_name),
    };
}

fn remove_bot_name_from_text(text: String) -> String {
    match text.starts_with("@") {
        true => return text.split("@Rusty Nanobot").nth(1).unwrap().to_string(),
        false => return text,
    }
}

fn add_account_to_database(acc: node::NewAccount, email: String, db_conn: &Mutex<Connection>) -> String { 
    match db::add_account(&db_conn, acc) {
        Ok(_) => format!("Account has been succesfully created, to check your balance type `!balance`"),
        Err(err) => format!("{}", err)
    }
}

pub fn rocket(db_conn: Mutex<Connection>) -> Rocket {
    Rocket::ignite()
        .manage(db_conn)
        .mount("/", routes![post_json, moo])
}
