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
    widgets: Vec<Widget>
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
struct Widget {
    image: Image
}

trait WidgetTrait {
    fn as_any(&self) -> &Any;
}

impl Serialize for Box<WidgetTrait> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> 
        where S: Serializer {
            return match self.as_any().downcast_ref::<Image>() {
                Some(res) => {
                        let mut widget_serializer = serializer.serialize_struct("Image", 1)?;
                        widget_serializer.serialize_field("imageUrl", &res.image_url)?;

                        widget_serializer.end()  
                    },
                None => {
                    let key_value: &KeyValue = match self.as_any().downcast_ref::<KeyValue>() {
                        Some(b) => b,
                        None => panic!("Unknown type!")
                    };

                    let mut widget_serializer = serializer.serialize_struct("KeyValue", 2)?;
                    widget_serializer.serialize_field("topLabel", &key_value.top_label)?;
                    widget_serializer.serialize_field("content", &key_value.content)?;

                    widget_serializer.end()  
                }
            };                
        }
}

impl WidgetTrait for Image {
    fn as_any(&self) -> &Any {
        self
    }
}

impl WidgetTrait for KeyValue {
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
    return Json(get_qr_code_response("asd".to_string()));
}

fn parse_text(text: String, user: Sender, db_conn: &Mutex<Connection>) -> ResponseMessage {
    return match remove_bot_name_from_text(text).trim() {
        "!help" => ResponseMessage { text: Some("Available commands: `!help` `!create_account` `!balance` `!deposit`".to_string()), cards: None },
        "!create_account" => match node::create_new_account() {
            Ok(acc) => ResponseMessage { text: Some(add_account_to_database(acc, user.email, db_conn)), cards: None },
            Err(err) => ResponseMessage { text: Some(format!("{}", err)), cards: None }
        },
        "!balance" => ResponseMessage { text: Some(get_balance(user.email, db_conn)), cards: None },
        "!deposit" => get_qr_code_response("text".to_string()),
        _ => ResponseMessage { text: Some(format!("Did not quite catch that, *{}*, type `!help` for help", user.display_name)), cards: None }
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

fn get_qr_code_response(text: String) -> ResponseMessage {
    ResponseMessage { 
            text: None, 
            cards: Some(
                vec![Card { sections: vec![
                    // Section { header: format!("Deposit"), widgets: vec![
                    //      Box::new(KeyValue { top_label: format!("To"), content: format!("user_name, email_address") }),
                    //      Box::new(KeyValue { top_label: format!("Wallet"), content: format!("wallet_address") })
                    //      ]},
                    Section { header: format!("Scan QR Code using Nano mobile wallet"), widgets: vec![ 
                        Widget { image: Image { image_url: format!("http://s2.quickmeme.com/img/d0/d073103e1d49fa4240967821f13b77afc73a18898d009023f3d8f9bc808f9122.jpg") } } 
                        ]}
                    ]}])
            }
}

pub fn rocket(db_conn: Mutex<Connection>) -> Rocket {
    Rocket::ignite()
        .manage(db_conn)
        .mount("/", routes![post_json, moo])
}
