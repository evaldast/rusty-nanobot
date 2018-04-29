use db;
use node;
use rocket::Rocket;
use rocket::State;
use rocket_contrib::Json;
use rusqlite::Connection;
use std::sync::Mutex;
use node::{Account, Balance};
use serde::ser::{Serialize, Serializer, SerializeStruct};
use std::any::Any;

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
    text: Option<String>,
    cards: Option<Vec<Card>>
}

#[derive(Serialize)]
struct Card {
    sections: Vec<Section>
}

#[derive(Serialize)]
struct Section {
    header: String,
    widgets: Vec<Box<Widget>>
}

#[derive(Serialize)]
struct Image {
    #[serde(rename = "imageUrl")]
    image_url: String
}

#[derive(Serialize)]
struct KeyValue {
    #[serde(rename = "topLabel")]
    top_label: String,

    content: String
}

#[derive(Serialize)]
struct ImageWidget {
    image: Image
}

#[derive(Serialize)]
struct KeyValueWidget {
    #[serde(rename = "keyValue")]
    key_value: KeyValue
}

trait Widget {
    fn as_any(&self) -> &Any;
}

impl Serialize for Box<Widget> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> 
        where S: Serializer {
            return match self.as_any().downcast_ref::<ImageWidget>() {
                Some(res) => {
                        let mut widget_serializer = serializer.serialize_struct("ImageWidget", 1)?;
                        widget_serializer.serialize_field("image", &res.image)?;

                        widget_serializer.end()  
                    },
                None => {
                    let key_value: &KeyValueWidget = match self.as_any().downcast_ref::<KeyValueWidget>() {
                        Some(b) => b,
                        None => panic!("Unknown type!")
                    };

                    let mut widget_serializer = serializer.serialize_struct("KeyValue", 1)?;
                    widget_serializer.serialize_field("keyValue", &key_value.key_value)?;

                    widget_serializer.end()  
                }
            };                
        }
}

impl Widget for ImageWidget {
    fn as_any(&self) -> &Any {
        self
    }
}

impl Widget for KeyValueWidget {
    fn as_any(&self) -> &Any {
        self
    }
}


#[post("/hello", format = "application/json", data = "<event>")]
fn post_json(db_conn: State<Mutex<Connection>>, event: Json<Event>) -> Json<ResponseMessage> {
    // println!("{:?}", &event.0);

    match event.0.event_type.trim() {
        "ADDED_TO_SPACE" => {
            return Json(ResponseMessage {
                text: Some(format!("Hello and thanks for adding me, *{}*. For help type `!help`", event.0.user.display_name)),
                cards: None
            })
        }
        "MESSAGE" => { 
            return Json(parse_text(event.0.message.text, event.0.user, &db_conn))
            }
        _ => {
            return Json(ResponseMessage {
                text: Some("Unsupported event".to_string()),
                cards: None
            })
        }
    };
}

#[get("/")]
fn moo() -> Json<ResponseMessage> {
    return Json(ResponseMessage { text: Some("swx".to_owned()), cards: None });
}

fn parse_text(text: String, user: Sender, db_conn: &Mutex<Connection>) -> ResponseMessage {
    return match remove_bot_name_from_text(text).trim() {
        "!help" => ResponseMessage { text: Some("Available commands: `!help` `!create_account` `!balance` `!deposit`".to_string()), cards: None },
        "!create_account" => ResponseMessage { text: Some(try_create_account(&user, &db_conn)), cards: None },
        "!balance" => ResponseMessage { text: Some(get_balance(user.email, db_conn)), cards: None },
        "!deposit" => get_deposit_response(&user, db_conn),
        _ => ResponseMessage { text: Some(format!("Did not quite catch that, *{}*, type `!help` for help", user.display_name)), cards: None }
    };
}

fn remove_bot_name_from_text(text: String) -> String {
    match text.starts_with("@") {
        true => return text.split("@Rusty Nanobot").nth(1).unwrap().to_string(),
        false => return text,
    }
}

fn try_create_account(user: &Sender, db_conn: &Mutex<Connection>) -> String {
    let has_account: bool = match db::get_account(db_conn, &user.email) {
        Ok(_) => true,
        Err(_) => false
    };

    if has_account {
        return "It seems that you already own an account".to_owned();
    }

    match node::create_new_account() {
        Ok(acc) => match db::add_account(db_conn, acc, String::from(&*user.email)) {
            Ok(_) => "Account has been succesfully created, to check your balance type `!balance`".to_owned(),
            Err(e) => e.to_string()
        },
        Err(e) => e.to_string()
    }
} 

fn add_account_to_database(acc: Account, email: String, db_conn: &Mutex<Connection>) -> String {
    match db::add_account(&db_conn, acc, email) {
        Ok(_) => format!("Account has been succesfully created, to check your balance type `!balance`"),
        Err(err) => format!("{}", err)
    }
}

fn get_balance(email: String, db_conn: &Mutex<Connection>) -> String {
    let acc:Account = match db::get_account(db_conn, &email) {
        Ok(a) => a,
        Err(err) => return format!("{}", err)
    };

    let bal:Balance = match node::get_balance(acc.account) {
        Ok(b) => b,
        Err(err) => return format!("{}", err)
    };

    return format!("Current balance: {}; Pending: {}", bal.balance, bal.pending);
}

fn get_deposit_response(user: &Sender, db_conn: &Mutex<Connection>) -> ResponseMessage {
    let acc:Account = match db::get_account(db_conn, &user.email) {
        Ok(a) => a,
        Err(err) => return ResponseMessage { text: Some(err.to_string()), cards: None }
    };

    ResponseMessage { 
            text: None, 
            cards: Some(
                vec![Card { sections: vec![
                    Section { header: format!("Deposit"), widgets: vec![
                         Box::new(KeyValueWidget { key_value: KeyValue { top_label: format!("To"), content: format!("{}, {}", user.display_name, user.email) } }),
                         Box::new(KeyValueWidget { key_value: KeyValue { top_label: format!("Wallet"), content: format!("{}", acc.account) } })
                         ]},
                    Section { header: format!("Scan QR Code using Nano mobile wallet"), widgets: vec![ 
                        Box::new(ImageWidget { image: Image { image_url: format!("https://api.qrserver.com/v1/create-qr-code/?data={}", acc.account) } })
                        ]}
                    ]}])
            }
}

pub fn rocket(db_conn: Mutex<Connection>) -> Rocket {
    Rocket::ignite()
        .manage(db_conn)
        .mount("/", routes![post_json, moo])
}
