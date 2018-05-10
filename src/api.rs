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
use regex::Regex;

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
    //println!("{:?}", &event.0);

    match event.0.event_type.trim() {
        "ADDED_TO_SPACE" => {
            return Json(ResponseMessage {
                text: Some(format!("Hello and thanks for adding me, *{}*. For help type `!help`", event.0.user.display_name)),
                cards: None
            })
        }
        "MESSAGE" => { 
            return Json(parse_text(&event.0.message.text, event.0.user, &db_conn))
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

fn parse_text(text: &str, user: Sender, db_conn: &Mutex<Connection>) -> ResponseMessage {
    return match remove_bot_name_from_text(text) {
        "!help" => ResponseMessage { text: Some("Available commands: `!help` `!create_account` `!balance` `!deposit` `!tip receiver_email amount`".to_string()), cards: None },
        "!create_account" => ResponseMessage { text: Some(try_create_account(&user.email, &db_conn).to_string()), cards: None },
        "!balance" => get_balance(&user.email, &db_conn),
        "!deposit" => get_deposit_response(&user, db_conn),
        t => match t.starts_with("!tip") {
            true => try_tip(&db_conn, &t),
            false => ResponseMessage { text: Some(format!("Did not quite catch that, *{}*, type `!help` for help", user.display_name)), cards: None }
        }
    };
}

fn remove_bot_name_from_text(text: &str) -> &str {
    match text.trim().starts_with("@") {
        true => text.split("@Rusty Nanobot").nth(1).unwrap(),
        false => text,
    }
}

fn try_create_account(user_email: &str, db_conn: &Mutex<Connection>) -> &'static str {
    let wallet: node::Wallet = match node::create_new_wallet() {
        Ok(w) => w,
        Err(_) => return "An error has occured attempting to create a wallet"
    }; 

    let key: node::Key = match node::create_new_key() {
        Ok(k) => k,
        Err(_) => return "An error has occured attempting to create a key"
    };

    match node::add_key_to_wallet(&wallet.wallet, &key.private) {
        Ok(_) => match db::add_account(db_conn, &key, String::from(&*user_email), wallet.wallet) {
            Ok(_) => "Account has been succesfully created, to check your balance type `!balance`",
            Err(_) => "An error has occured attempting to create an account"
        },
        Err(_) => "An error has occured attempting to add key to a wallet"
    }
}

fn try_get_account(user_email: &str, db_conn: &Mutex<Connection>) -> Result<Account, String> {
    let has_account: bool = match db::get_account(db_conn, user_email) {
        Ok(_) => true,
        Err(_) => false
    };

    if !has_account {
        return match try_create_account(&user_email, db_conn) {
            "Account has been succesfully created, to check your balance type `!balance`" => {
                return match db::get_account(db_conn, user_email) {
                    Ok(a) => Ok(a),
                    Err(_) => Err("An error has occured".to_string())
                }
            },
            _ => Err("An error has occured".to_string())
        }
    };

    return match db::get_account(db_conn, user_email) {
        Ok(a) => Ok(a),
        Err(_) => Err("An error has occured".to_string())
    }
}

fn get_balance(user_email: &str, db_conn: &Mutex<Connection>) -> ResponseMessage {
    let acc:Account = match try_get_account(user_email, db_conn) {
        Ok(a) => a,
        Err(_) => return ResponseMessage { text: Some("An error has occured fetching the account".to_string()), cards: None }
    };

    let bal:Balance = match node::get_balance(acc.account) {
        Ok(b) => b,
        Err(_) => return ResponseMessage { text: Some("An error has occured fetching the balance".to_string()), cards: None }
    };

    ResponseMessage { 
            text: None, 
            cards: Some(
                vec![Card { sections: vec![
                    Section { header: format!("Balance"), widgets: vec![
                         Box::new(KeyValueWidget { key_value: KeyValue { top_label: "Current".to_string(), content: bal.balance } }),
                         Box::new(KeyValueWidget { key_value: KeyValue { top_label: "Pending".to_string(), content: bal.pending } })
                         ]}
                    ]}])
            }
}

fn get_deposit_response(user: &Sender, db_conn: &Mutex<Connection>) -> ResponseMessage {
    let acc:Account = match try_get_account(&user.email, db_conn) {
        Ok(a) => a,
        Err(_) => return ResponseMessage { text: Some("There was an error fetching the account".to_string()), cards: None }
    };

    ResponseMessage { 
            text: None, 
            cards: Some(
                vec![Card { sections: vec![
                    Section { header: format!("Deposit"), widgets: vec![
                         Box::new(KeyValueWidget { key_value: KeyValue { top_label: format!("To"), content: format!("{}", user.email) } }),
                         Box::new(KeyValueWidget { key_value: KeyValue { top_label: format!("Wallet"), content: format!("{}", acc.account) } })
                         ]},
                    Section { header: format!("Scan QR Code using Nano mobile wallet"), widgets: vec![ 
                        Box::new(ImageWidget { image: Image { image_url: format!("https://api.qrserver.com/v1/create-qr-code/?data={}", acc.account) } })
                        ]}
                    ]}])
            }
}

fn try_tip(db_conn: &Mutex<Connection>, text_args: &str) -> ResponseMessage {
    let tip_args: (&str, &str) = match parse_tip_arguments(text_args) {
        Ok(a) => a,
        Err(e) => return ResponseMessage { text: Some(e), cards: None }
    };
    
    let acc:Account = match try_get_account(tip_args.0, db_conn) {
        Ok(a) => a,
        Err(_) => return ResponseMessage { text: Some("There was an error fetching the account".to_string()), cards: None }
    };

    ResponseMessage { 
            text: None, 
            cards: Some(
                vec![Card { sections: vec![
                    Section { header: format!("Tip"), widgets: vec![
                         Box::new(KeyValueWidget { key_value: KeyValue { top_label: format!("To"), content: format!("{}", &tip_args.0) } }),
                         Box::new(KeyValueWidget { key_value: KeyValue { top_label: format!("Wallet"), content: format!("{}", acc.account) } })
                         ]},
                    Section { header: format!("Scan QR Code using Nano mobile wallet"), widgets: vec![ 
                        Box::new(ImageWidget { image: Image { image_url: format!("https://api.qrserver.com/v1/create-qr-code/?data={}", acc.account) } })
                        ]}
                    ]}])
            }
}

fn parse_tip_arguments(text_args: &str) -> Result<(&str, &str), String> {
    let mut args = text_args.split_whitespace();

    let email: &str = match args.nth(1) {
        Some(a) => {
            match Regex::new(r"^[a-zA-Z0-9_.+-]+@(?:(?:[a-zA-Z0-9-]+.)?[a-zA-Z]+.)?(visma).com$").unwrap().is_match(a) {
                true => a,
                false => return Err("Could not parse email address. Must have @visma.com format".to_string())
            }
        },
        _ => return Err("No email supplied".to_string())
    };

    let amount: &str = match args.next() {
        Some(a) => {
            match Regex::new(r"^[1-9][0-9]*$").unwrap().is_match(a) {
                true => a,
                false => return Err("Could not parse amount".to_string())
            }
        },
        _ => return Err("No amount supplied".to_string())
    };

    println!("{} {}", email, amount);

    return Ok((email, amount));
}

pub fn rocket(db_conn: Mutex<Connection>) -> Rocket {
    Rocket::ignite()
        .manage(db_conn)
        .mount("/", routes![post_json, moo])
}
