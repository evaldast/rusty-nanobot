use db;
use node;
use node::{Account, Balance};
use rocket::{Rocket, State};
use rocket_contrib::Json;
use rusqlite::Connection;
use std::sync::Mutex;
use std::ops::Deref;
use serde::ser::{Serialize, Serializer, SerializeStruct};
use std::any::Any;
use regex::Regex;
use futures::{Future, Stream};
use hyper::{Client, Method, Request, Uri, header, client};
use tokio_core::reactor::Core;
use std::error::Error;
use serde_json;
use chrono::{Duration, Utc, DateTime};
use hyper_tls::HttpsConnector;
use api::coinmarketcap::{get_nano_price_in_euros};
use api::hangouts;
use api::teams;

#[post("/hangouts", format = "application/json", data = "<event>")]
fn hangouts(db_conn: State<Mutex<Connection>>, event: Json<hangouts::Event>) -> Json<hangouts::ResponseMessage> {
    Json(hangouts::handle_message(&db_conn, event.0))
}

#[post("/teams", format = "application/json", data = "<activity>")]
fn teams(activity: Json<teams::Activity>, bearer_token: State<Mutex<teams::TeamsToken>>) {
    teams::handle_message(activity.0, &bearer_token)
}

#[get("/")]
fn moo() -> Json {
    match get_nano_price_in_euros() {
        Ok(r) => Json(json!(r)),
        Err(_) => Json(json!("Oooops"))
    }
}

pub fn rocket(db_conn: Mutex<Connection>) -> Rocket {
    Rocket::ignite()
        .manage(db_conn)
        .manage(Mutex::new(teams::TeamsToken { token: "initial_token".to_string(), expire_date: Utc::now() }))
        .mount("/", routes![hangouts, teams, moo])
}
