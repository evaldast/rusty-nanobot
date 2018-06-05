use rocket::{Rocket, State};
use rocket_contrib::Json;
use rusqlite::Connection;
use std::sync::Mutex;
use chrono::{Utc};
use api::coinmarketcap::{get_nano_price_in_euros};
use api::hangouts;
use api::teams;

#[post("/hangouts", format = "application/json", data = "<event>")]
fn hangouts(db_conn: State<Mutex<Connection>>, event: Json<hangouts::Event>) -> Json<hangouts::ResponseMessage> {
    Json(hangouts::handle_message(&db_conn, event.0))
}

#[post("/teams", format = "application/json", data = "<activity>")]
fn teams(activity: Json<teams::Activity>, bearer_token: State<Mutex<teams::TeamsToken>>) -> Json {
    println!("{:?}", activity.0);

    match teams::handle_message(activity.0, &bearer_token) {
        Ok(_) => Json(json!(())),
        Err(_) => Json(json!("woops"))
    }
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
        .manage(Mutex::new(teams::TeamsToken { value: "initial_token".to_string(), expire_date: Utc::now() }))
        .mount("/", routes![hangouts, teams, moo])
}
