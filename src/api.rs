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
use hyper::Client;
use futures::{Future, Stream};
use hyper::{Method, Request, Chunk};
use tokio_core::reactor::Core;
use std::error::Error;
use serde_json;
use serde_json::Value;

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
            match self.as_any().downcast_ref::<ImageWidget>() {
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
            }                
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

#[derive(Deserialize)]
struct CoinmarketcapInfo {
    data: CoinmarketcapData,
    metadata: CoinmarketcapMetadata
}

#[derive(Deserialize)]
struct CoinmarketcapData {
    id: u16,
    name: String,
    symbol: String,
    website_slug: String,
    rank: u16,
    circulating_supply: f64,
    total_supply: f64,
    max_supply: f64,
    quotes: CoinmarketcapQuotes,
    last_updated: u64
}

#[derive(Deserialize)]
struct CoinmarketcapMetadata {
    timestamp: u64,
    error: Option<String>
}

#[derive(Deserialize)]
struct CoinmarketcapQuotes {
    USD: CoinmarketcapQuote,
    EUR: CoinmarketcapQuote
}

#[derive(Deserialize)]
struct CoinmarketcapQuote {
    price: f32,
    volume_24h: f64,
    market_cap: f64,
    percent_change_1h: f32,
    percent_change_24h: f32,
    percent_change_7d: f32
}


#[post("/hello", format = "application/json", data = "<event>")]
fn post_json(db_conn: State<Mutex<Connection>>, event: Json<Event>) -> Json<ResponseMessage> {
    //println!("{:?}", &event.0);

    match event.0.event_type.trim() {
        "ADDED_TO_SPACE" => {
            Json(ResponseMessage {
                text: Some(format!("Hello and thanks for adding me, *{}*. For help type `!help`", event.0.user.display_name)),
                cards: None
            })
        }
        "MESSAGE" => { 
            Json(parse_text(&event.0.message.text, &event.0.user, &db_conn))
            }
        _ => {
            Json(ResponseMessage {
                text: Some("Unsupported event".to_string()),
                cards: None
            })
        }
    }
}

#[get("/")]
fn moo() -> Json<Value> {

    let mut core = Core::new().unwrap();
    let client = ::hyper::Client::configure()
        .connector(::hyper_tls::HttpsConnector::new(4, &core.handle()).unwrap())
        .build(&core.handle());

    let uri = "https://api.coinmarketcap.com/v2/ticker/1567/?convert=EUR".parse().unwrap();
    let work = client.get(uri).and_then(|res| {
        res.body().concat2().and_then(move |body| {
            let v: CoinmarketcapInfo = serde_json::from_slice(&body).unwrap();
            println!("current IP address is {}", v.data.last_updated);
            Ok(())
        })
    });

    Json(json!(core.run(work).unwrap()))

    //Json(ResponseMessage { text: Some("swx".to_string()), cards: None })
}

fn parse_text(text: &str, user: &Sender, db_conn: &Mutex<Connection>) -> ResponseMessage {
    match remove_bot_name_from_text(text).trim() {
        "!help" => ResponseMessage { text: Some("Available commands: `!balance` `!deposit` `!tip receiver_email amount` `!withdraw wallet_address`".to_string()), cards: None },        
        "!balance" => get_balance(&user.email, &db_conn),
        "!deposit" => get_deposit_response(&user, db_conn),
        t => if t.starts_with("!tip") { 
                try_tip(&db_conn, &t, &user.email)
            }
            else if t.starts_with("!withdraw") {
                ResponseMessage { text: Some("Not implemented yet".to_string()), cards: None }
            } 
            else {
                ResponseMessage { text: Some(format!("Did not quite catch that, *{}*, type `!help` for help", user.display_name)), cards: None }
            }
    }
}

fn remove_bot_name_from_text(text: &str) -> &str {
    if text.trim().starts_with("@Rusty Nanobot") {
        text.split("@Rusty Nanobot").nth(1).unwrap()
    }
    else {
        text
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
        Ok(_) => match db::add_account(db_conn, &key, user_email, &wallet.wallet) {
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

    match db::get_account(db_conn, user_email) {
        Ok(a) => Ok(a),
        Err(_) => Err("An error has occured".to_string())
    }
}

fn get_balance(user_email: &str, db_conn: &Mutex<Connection>) -> ResponseMessage {
    let acc: Account = match try_get_account(user_email, db_conn) {
        Ok(a) => a,
        Err(_) => return ResponseMessage { text: Some("An error has occured fetching the account".to_string()), cards: None }
    };

    let bal: Balance = match node::get_balance(acc.account) {
        Ok(b) => b,
        Err(_) => return ResponseMessage { text: Some("An error has occured fetching the balance".to_string()), cards: None }
    };

    let converted_balances: (String, String) = (
        match convert_raw_to_nano(&bal.balance) {
            Ok(b) => b.to_string(),
            Err(_) => return ResponseMessage { text: Some("An error has occured converting the balance".to_string()), cards: None }
        },
        match convert_raw_to_nano(&bal.pending) {
            Ok(p) => p.to_string(),
            Err(_) => return ResponseMessage { text: Some("An error has occured fetching the balance".to_string()), cards: None } 
        }
    );

    ResponseMessage { 
            text: None, 
            cards: Some(
                vec![Card { sections: vec![
                    Section { header: "Balance".to_string(), widgets: vec![
                         Box::new(KeyValueWidget { key_value: KeyValue { top_label: "Current".to_string(), content: format!("{} NANO, €", converted_balances.0) } }),
                         Box::new(KeyValueWidget { key_value: KeyValue { top_label: "Pending".to_string(), content: format!("{} NANO, €", converted_balances.1) } })
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
                    Section { header: "Deposit".to_string(), widgets: vec![
                         Box::new(KeyValueWidget { key_value: KeyValue { top_label: "To".to_string(), content: user.email.to_owned() } }),
                         Box::new(KeyValueWidget { key_value: KeyValue { top_label: "Wallet".to_string(), content: acc.account.to_owned() } })
                         ]},
                    Section { header: "Scan QR Code using Nano mobile wallet".to_string(), widgets: vec![ 
                        Box::new(ImageWidget { image: Image { image_url: format!("https://api.qrserver.com/v1/create-qr-code/?data={}", acc.account) } })
                        ]}
                    ]}])
            }
}

fn try_tip(db_conn: &Mutex<Connection>, text_args: &str, sender_email: &str) -> ResponseMessage {
    let tip_args: (&str, &str) = match parse_tip_arguments(text_args) {
        Ok(a) => a,
        Err(e) => return ResponseMessage { text: Some(e), cards: None }
    };
    
    let receiver_acc: Account = match try_get_account(tip_args.0, db_conn) {
        Ok(a) => a,
        Err(_) => return ResponseMessage { text: Some("There was an error fetching the receiver account".to_string()), cards: None }
    };

    let sender_acc: Account = match try_get_account(sender_email, db_conn) {
        Ok(a) => a,
        Err(_) => return ResponseMessage {text: Some("There was an error fetching the sender account".to_string()), cards: None}
    };

    match node::send(&sender_acc.wallet, &sender_acc.account, &receiver_acc.account, tip_args.1) {
        Ok(_) => ResponseMessage { 
            text: None, 
            cards: Some(
                vec![Card { sections: vec![
                    Section { header: "Tip sent!".to_string(), widgets: vec![
                        Box::new(KeyValueWidget { key_value: KeyValue { top_label: "From".to_string(), content: sender_email.to_owned() } }),
                         Box::new(KeyValueWidget { key_value: KeyValue { top_label: "To".to_string(), content: tip_args.0.to_owned() } }),
                         Box::new(KeyValueWidget { key_value: KeyValue { top_label: "Wallet".to_string(), content: receiver_acc.account.to_owned() } }),
                         Box::new(KeyValueWidget { key_value: KeyValue { top_label: "Amount".to_string(), content: tip_args.1.to_owned() } })
                         ]}
                    ]}])
            },
        Err(_) => ResponseMessage {text: Some("There was an error sending the tip".to_string()), cards: None}
    }
}

fn parse_tip_arguments(text_args: &str) -> Result<(&str, &str), String> {
    let mut args = text_args.split_whitespace();

    let email: &str = match args.nth(1) {
        Some(a) => {
            if Regex::new(r"^[a-zA-Z0-9_.+-]+@(?:(?:[a-zA-Z0-9-]+.)?[a-zA-Z]+.)?(visma).com$").unwrap().is_match(a) {
                a 
            } 
            else {
                return Err("Could not parse email address. Must have @visma.com format".to_string())
            }
        },
        _ => return Err("No email supplied".to_string())
    };

    let amount: &str = match args.next() {
        Some(a) => {
            if Regex::new(r"^[1-9][0-9]*$").unwrap().is_match(a) {
                a
            } 
            else {
                return Err("Could not parse amount".to_string())
            }
        },
        _ => return Err("No amount supplied".to_string())
    };

    Ok((email, amount))
}

fn convert_raw_to_nano(raw_amount: &str) -> Result<u128, String> {
    match u128::from_str_radix(raw_amount, 10) {
        Ok(a) => Ok(a / 1_000_000_000_000_000_000_000_000),
        Err(_) => Err("Error converting raw to nano".to_string())
    } 
}

fn convert_raw_from_nano(nano_amount: &str) -> Result<u128, String> {
    match u128::from_str_radix(nano_amount, 10) {
        Ok(a) => Ok(a * 1_000_000_000_000_000_000_000_000),
        Err(_) => Err("Error converting nano to raw".to_string())
    } 
}

fn get_nano_price_in_euros() -> Result<Chunk, Box<Error>> {
    let mut core = Core::new()?;
    let client = Client::new(&core.handle());
    let uri = "https://api.coinmarketcap.com/v2/ticker/1567/?convert=EUR".parse()?;
    let req = Request::new(Method::Get, uri);

    let post = client.request(req).and_then(|res| {
        res.body().concat2()
    });

    let response = core.run(post).unwrap();

    Ok(response)
}

pub fn rocket(db_conn: Mutex<Connection>) -> Rocket {
    Rocket::ignite()
        .manage(db_conn)
        .mount("/", routes![post_json, moo])
}
