use db;
use node;
use regex::Regex;
use rusqlite::Connection;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use std::any::Any;
use std::error::Error;
use std::sync::Mutex;

#[derive(Deserialize, Debug)]
pub struct Event {
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
pub struct ResponseMessage {
    text: Option<String>,
    cards: Option<Vec<Card>>,
}

#[derive(Serialize)]
struct Card {
    sections: Vec<Section>,
}

#[derive(Serialize)]
struct Section {
    header: String,
    widgets: Vec<Box<Widget>>,
}

#[derive(Serialize)]
struct Image {
    #[serde(rename = "imageUrl")]
    image_url: String,
}

#[derive(Serialize)]
struct KeyValue {
    #[serde(rename = "topLabel")]
    top_label: String,

    content: String,
}

#[derive(Serialize)]
struct ImageWidget {
    image: Image,
}

#[derive(Serialize)]
struct KeyValueWidget {
    #[serde(rename = "keyValue")]
    key_value: KeyValue,
}

trait Widget {
    fn as_any(&self) -> &Any;
}

impl Serialize for Box<Widget> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.as_any().downcast_ref::<ImageWidget>() {
            Some(res) => {
                let mut widget_serializer = serializer.serialize_struct("ImageWidget", 1)?;
                widget_serializer.serialize_field("image", &res.image)?;

                widget_serializer.end()
            }
            None => {
                let key_value: &KeyValueWidget =
                    match self.as_any().downcast_ref::<KeyValueWidget>() {
                        Some(b) => b,
                        None => panic!("Unknown type!"),
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

pub fn handle_message(db_conn: &Mutex<Connection>, event: Event) -> ResponseMessage {
    match event.event_type.trim() {
        "ADDED_TO_SPACE" => ResponseMessage {
            text: Some(format!(
                "Hello and thanks for adding me, *{}*. For help type `!help`",
                event.user.display_name
            )),
            cards: None,
        },
        "MESSAGE" => parse_text(&event.message.text, &event.user, db_conn),
        _ => ResponseMessage {
            text: Some("Unsupported event".to_string()),
            cards: None,
        },
    }
}

fn parse_text(text: &str, user: &Sender, db_conn: &Mutex<Connection>) -> ResponseMessage {
    match remove_bot_name_from_text(text).trim() {
        "!help" => ResponseMessage { text: Some("Available commands: `!balance` `!deposit` `!tip receiver_email amount` `!withdraw wallet_address`".to_string()), cards: None },        
        "!balance" => get_balance(&user.email, db_conn),
        "!deposit" => get_deposit_response(&user, db_conn),
        t => if t.starts_with("!tip") { 
                try_tip(db_conn, &t, &user.email)
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
        match text.split("@Rusty Nanobot").nth(1) {
            Some(t) => t,
            _ => text,
        }
    } else {
        text
    }
}

fn get_deposit_response(user: &Sender, db_conn: &Mutex<Connection>) -> ResponseMessage {
    let acc: node::Account = match try_get_account(&user.email, db_conn) {
        Ok(a) => a,
        Err(_) => {
            return ResponseMessage {
                text: Some("There was an error fetching the account".to_string()),
                cards: None,
            }
        }
    };

    ResponseMessage {
        text: None,
        cards: Some(vec![Card {
            sections: vec![
                Section {
                    header: "Deposit".to_string(),
                    widgets: vec![
                        Box::new(KeyValueWidget {
                            key_value: KeyValue {
                                top_label: "To".to_string(),
                                content: user.email.to_owned(),
                            },
                        }),
                        Box::new(KeyValueWidget {
                            key_value: KeyValue {
                                top_label: "Wallet".to_string(),
                                content: acc.account.to_owned(),
                            },
                        }),
                    ],
                },
                Section {
                    header: "Scan QR Code using Nano mobile wallet".to_string(),
                    widgets: vec![Box::new(ImageWidget {
                        image: Image {
                            image_url: format!(
                                "https://api.qrserver.com/v1/create-qr-code/?data={}",
                                acc.account
                            ),
                        },
                    })],
                },
            ],
        }]),
    }
}

fn try_tip(db_conn: &Mutex<Connection>, text_args: &str, sender_email: &str) -> ResponseMessage {
    let tip_args: (&str, &str) = match parse_tip_arguments(text_args) {
        Ok(a) => a,
        Err(e) => {
            return ResponseMessage {
                text: Some(e),
                cards: None,
            }
        }
    };

    let receiver_acc: node::Account = match try_get_account(tip_args.0, db_conn) {
        Ok(a) => a,
        Err(_) => {
            return ResponseMessage {
                text: Some("There was an error fetching the receiver account".to_string()),
                cards: None,
            }
        }
    };

    let sender_acc: node::Account = match try_get_account(sender_email, db_conn) {
        Ok(a) => a,
        Err(_) => {
            return ResponseMessage {
                text: Some("There was an error fetching the sender account".to_string()),
                cards: None,
            }
        }
    };

    match node::send(
        &sender_acc.wallet,
        &sender_acc.account,
        &receiver_acc.account,
        tip_args.1,
    ) {
        Ok(_) => ResponseMessage {
            text: None,
            cards: Some(vec![Card {
                sections: vec![Section {
                    header: "Tip sent!".to_string(),
                    widgets: vec![
                        Box::new(KeyValueWidget {
                            key_value: KeyValue {
                                top_label: "From".to_string(),
                                content: sender_email.to_owned(),
                            },
                        }),
                        Box::new(KeyValueWidget {
                            key_value: KeyValue {
                                top_label: "To".to_string(),
                                content: tip_args.0.to_owned(),
                            },
                        }),
                        Box::new(KeyValueWidget {
                            key_value: KeyValue {
                                top_label: "Wallet".to_string(),
                                content: receiver_acc.account.to_owned(),
                            },
                        }),
                        Box::new(KeyValueWidget {
                            key_value: KeyValue {
                                top_label: "Amount".to_string(),
                                content: tip_args.1.to_owned(),
                            },
                        }),
                    ],
                }],
            }]),
        },
        Err(_) => ResponseMessage {
            text: Some("There was an error sending the tip".to_string()),
            cards: None,
        },
    }
}

fn parse_tip_arguments(text_args: &str) -> Result<(&str, &str), String> {
    let mut args = text_args.split_whitespace();

    let email: &str = match args.nth(1) {
        Some(email) => {
            if match validate_email_address(email) {
                Ok(r) => r,
                Err(_) => false,
            } {
                email
            } else {
                return Err("Could not parse email address".to_string());
            }
        }
        _ => return Err("No email supplied".to_string()),
    };

    let amount: &str = match args.next() {
        Some(amount) => {
            if match validate_tip_amont(amount) {
                Ok(r) => r,
                Err(_) => false,
            } {
                amount
            } else {
                return Err("Could not parse amount".to_string());
            }
        }
        _ => return Err("No amount supplied".to_string()),
    };

    Ok((email, amount))
}

fn try_get_account(user_email: &str, db_conn: &Mutex<Connection>) -> Result<node::Account, String> {
    let has_account: bool = match db::get_account_hangouts(db_conn, user_email) {
        Ok(_) => true,
        Err(_) => false,
    };

    if !has_account {
        return match try_create_account(&user_email, db_conn) {
            "Account has been succesfully created, to check your balance type `!balance`" => {
                return match db::get_account_hangouts(db_conn, user_email) {
                    Ok(a) => Ok(a),
                    Err(_) => Err("An error has occured".to_string()),
                }
            }
            _ => Err("An error has occured".to_string()),
        };
    };

    match db::get_account_hangouts(db_conn, user_email) {
        Ok(a) => Ok(a),
        Err(_) => Err("An error has occured".to_string()),
    }
}

fn try_create_account(user_email: &str, db_conn: &Mutex<Connection>) -> &'static str {
    let wallet: node::Wallet = match node::create_new_wallet() {
        Ok(w) => w,
        Err(_) => return "An error has occured attempting to create a wallet",
    };

    let key: node::Key = match node::create_new_key() {
        Ok(k) => k,
        Err(_) => return "An error has occured attempting to create a key",
    };

    match node::add_key_to_wallet(&wallet.wallet, &key.private) {
        Ok(_) => match db::add_account_hangouts(db_conn, &key, user_email, &wallet.wallet) {
            Ok(_) => "Account has been succesfully created, to check your balance type `!balance`",
            Err(_) => "An error has occured attempting to create an account",
        },
        Err(_) => "An error has occured attempting to add key to a wallet",
    }
}

fn get_balance(user_email: &str, db_conn: &Mutex<Connection>) -> ResponseMessage {
    let acc: node::Account = match try_get_account(user_email, db_conn) {
        Ok(a) => a,
        Err(_) => {
            return ResponseMessage {
                text: Some("An error has occured fetching the account".to_string()),
                cards: None,
            }
        }
    };

    let bal: node::Balance = match node::get_balance(acc.account) {
        Ok(b) => b,
        Err(_) => {
            return ResponseMessage {
                text: Some("An error has occured fetching the balance".to_string()),
                cards: None,
            }
        }
    };

    let converted_balances: (String, String) = (
        match convert_raw_to_nano(&bal.balance) {
            Ok(b) => b.to_string(),
            Err(_) => {
                return ResponseMessage {
                    text: Some("An error has occured converting the balance".to_string()),
                    cards: None,
                }
            }
        },
        match convert_raw_to_nano(&bal.pending) {
            Ok(p) => p.to_string(),
            Err(_) => {
                return ResponseMessage {
                    text: Some("An error has occured fetching the balance".to_string()),
                    cards: None,
                }
            }
        },
    );

    ResponseMessage {
        text: None,
        cards: Some(vec![Card {
            sections: vec![Section {
                header: "Balance".to_string(),
                widgets: vec![
                    Box::new(KeyValueWidget {
                        key_value: KeyValue {
                            top_label: "Current".to_string(),
                            content: format!("{} NANO, €", converted_balances.0),
                        },
                    }),
                    Box::new(KeyValueWidget {
                        key_value: KeyValue {
                            top_label: "Pending".to_string(),
                            content: format!("{} NANO, €", converted_balances.1),
                        },
                    }),
                ],
            }],
        }]),
    }
}

fn validate_email_address(email: &str) -> Result<bool, Box<Error>> {
    Ok(Regex::new(r"^[A-Z0-9._%+-]+@[A-Z0-9.-]+\\.[A-Z]{2,6}$")?.is_match(email))
}

fn validate_tip_amont(amount: &str) -> Result<bool, Box<Error>> {
    Ok(Regex::new(r"^[1-9][0-9]*$")?.is_match(amount))
}

fn convert_raw_to_nano(raw_amount: &str) -> Result<u128, String> {
    match u128::from_str_radix(raw_amount, 10) {
        Ok(a) => Ok(a / 1_000_000_000_000_000_000_000_000),
        Err(_) => Err("Error converting raw to nano".to_string()),
    }
}

fn convert_raw_from_nano(nano_amount: &str) -> Result<u128, String> {
    match u128::from_str_radix(nano_amount, 10) {
        Ok(a) => Ok(a * 1_000_000_000_000_000_000_000_000),
        Err(_) => Err("Error converting nano to raw".to_string()),
    }
}
