use api::coinmarketcap::get_nano_price_in_euros;
use api::hangouts;
use api::teams;
use chrono::Utc;
use rocket::{Rocket, State};
use rocket_contrib::Json;
use rusqlite::Connection;
use std::sync::Mutex;

#[post("/hangouts", format = "application/json", data = "<event>")]
fn hangouts(
    db_conn: State<Mutex<Connection>>,
    event: Json<hangouts::Event>,
) -> Json<hangouts::ResponseMessage> {
    Json(hangouts::handle_message(&db_conn, event.0))
}

#[post("/teams", format = "application/json", data = "<activity>")]
fn teams(
    activity: Json<teams::Activity>,
    bearer_token: State<Mutex<teams::TeamsToken>>,
    db_conn: State<Mutex<Connection>>,
) {
    println!("{:?}", activity.0);

    match teams::handle_message(activity.0, &bearer_token, &db_conn) {
        Ok(_) => println!("Teams success"),
        Err(err) => println!("{}", err),
    }
}

#[get("/")]
fn moo() -> Json {
    match get_nano_price_in_euros() {
        Ok(r) => Json(json!(r)),
        Err(_) => Json(json!("Oooops")),
    }
}

pub fn rocket(db_conn: Mutex<Connection>) -> Rocket {
    Rocket::ignite()
        .manage(db_conn)
        .manage(Mutex::new(teams::TeamsToken {
            value: "initial_token".to_string(),
            expire_date: Utc::now(),
        }))
        .mount("/", routes![hangouts, teams, moo])
}
