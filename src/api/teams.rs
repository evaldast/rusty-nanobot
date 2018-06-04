use std::error::Error;
use hyper::{Client, Method, Request, Uri, header, client};
use serde_json;
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Core;
use futures::{Future, Stream};
use std::any::Any;
use rocket_contrib::Json;
use std::sync::Mutex;
use rusqlite::Connection;
use serde::ser::{Serialize, Serializer, SerializeStruct};
use node;
use regex::Regex;
use db;
use chrono::{Duration, Utc, DateTime};

#[derive(Deserialize, Debug)]
pub struct Activity {
    #[serde(rename = "type")]
    activity_type: String,

    id: String,
    timestamp: String,

    #[serde(rename = "serviceUrl")]
    service_url: String,

    #[serde(rename = "channelId")]
    channel_id: String,

    #[serde(default)]
    from: From,

    #[serde(default)]
    conversation: Conversation,

    #[serde(default)]
    recipient: Recipient,

    text: String
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct From {
    id: String,

    #[serde(default)]
    name: String
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Conversation {
    id: String,

    #[serde(default)]
    name: String
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct Recipient {
    id: String,

    #[serde(default)]
    name: String
}

#[derive(Serialize)]
struct TeamsResponse {
    #[serde(rename = "type")]
    response_type: String,

    from: From,
    conversation: Conversation,
    recipient: Recipient,
    text: String,
    replyToId: String
}

struct TeamsConfig {
    client_id: String,
    client_secret: String
}

pub struct TeamsToken {
    token: String,
    expire_date: DateTime<Utc>
}

#[derive(Deserialize, Debug)]
struct TokenResponse {
    token_type: String,
    expires_in: u16,
    ext_expires_in: u16,
    access_token: String,
}

#[derive(Deserialize, Debug)]
struct TokenError {
    error: String,
    error_description: String,
    error_codes: Vec<u16>,
    timestamp: String,
    trace_id: String,
    correlation_id: String
}

pub fn handle_message(activity: Activity, bearer_token: &Mutex<TeamsToken>) {
    println!("Refreshing token");

    refresh_teams_bearer_token(bearer_token);

    let state = bearer_token.lock().expect("Could not lock mutex");
    let teams_token = state.deref();

    println!("{}", teams_token.token);

    //auajFVRL55[[pylEWN522*!

    let mut core = Core::new().unwrap();
    let client = Client::configure()
        .connector(HttpsConnector::new(4, &core.handle()).unwrap())
        .build(&core.handle());
    let uri = format!("https://webchat.botframework.com/v3/conversations/{}/activities/{}", activity.conversation.id, activity.id).parse().unwrap();
    let mut req = Request::new(Method::Post, uri);

    println!("creating core and request");

    let teams_response = TeamsResponse {
        response_type: "message".to_string(),
        from: From { id: activity.recipient.id, name: activity.recipient.name },
        conversation: Conversation { id: activity.conversation.id, name: activity.conversation.name },
        recipient: Recipient { id: activity.from.id, name: activity.from.name },
        text: "Hi from Rusty".to_string(),
        replyToId: activity.id
    };

    let json = serde_json::to_string(&teams_response).unwrap();

    println!("creating teams response");

    req.headers_mut().set(header::ContentType::json());
    req.headers_mut().set(header::ContentLength(json.len() as u64));
    req.headers_mut().set(header::Authorization(header::Bearer { token: teams_token.token.to_string() }));
    req.set_body(json);

    println!("setting headers");

    let post = client.request(req).and_then(|res| {
        res.body().concat2()
    });

    println!("creating post");

    core.run(post);

    println!("done");
}

fn refresh_teams_bearer_token(teams_token: &Mutex<TeamsToken>) -> Result<(), Box<Error>> {
    let mut state = teams_token.lock().expect("Could not lock mutex");

    if state.deref().expire_date >= Utc::now() {
        return Ok(())
    }

    let mut core = Core::new()?;
    let client = get_https_client(&core)?;
    let uri = "https://login.microsoftonline.com/botframework.com/oauth2/v2.0/token".parse()?;
    let mut req = Request::new(Method::Post, uri);
    let body = "grant_type=client_credentials&client_id=88768615-5b58-4cd2-a6d8-a51bbec10126&client_secret=auajFVRL55[[pylEWN522*!&scope=https%3A%2F%2Fapi.botframework.com%2F.default";

    req.headers_mut().set(header::ContentLength(body.len() as u64));
    req.set_body(body);

    let post = client.request(req).and_then(|res| {
        res.body().concat2()
    });

    let response: TokenResponse = serde_json::from_slice(&core.run(post)?)?;

    *state = TeamsToken { token: response.access_token, expire_date: Utc::now() + Duration::seconds(response.expires_in as i64) };

    Ok(())
}

fn get_https_client(core: &Core) -> Result<Client<HttpsConnector<client::HttpConnector>>, Box<Error>> {
    let client = ::hyper::Client::configure()
        .connector(::hyper_tls::HttpsConnector::new(4, &core.handle())?)
        .build(&core.handle());
    
    Ok(client)
}