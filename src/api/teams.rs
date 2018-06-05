use std::error::Error;
use hyper::{Client, Method, Request, header, client};
use serde_json;
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Core;
use futures::{Future, Stream};
use std::sync::Mutex;
use chrono::{Duration, Utc, DateTime};
use erased_serde;

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
    text: Option<String>,
    attachments: Vec<Attachment>,

    #[serde(rename = "replyToId")]
    reply_to_id: String
}

#[derive(Serialize)]
struct TeamsResponseAdaptive {
    #[serde(rename = "type")]
    response_type: String,

    from: From,
    conversation: Conversation,
    recipient: Recipient,
    text: Option<String>,
    attachments: Vec<AttachmentAdaptive>,

    #[serde(rename = "replyToId")]
    reply_to_id: String
}

#[derive(Serialize)]
struct Attachment {
    #[serde(rename = "contentType")]
    content_type: String,

    content: AttachmentContent
}

#[derive(Serialize)]
struct AttachmentContent {
    title: String,
    subtitle: String,
    text: String,
    images: Vec<AttachmentImage>,
    buttons: Vec<AttachmentButton>
}

#[derive(Serialize)] 
struct AttachmentImage {
    url: String,
    alt: String
}

#[derive(Serialize)]
struct AttachmentButton {
    #[serde(rename = "type")]
    button_type: String,

    title: String,
    value: String
}

#[derive(Serialize)]
struct AttachmentAdaptive {
    #[serde(rename = "contentType")]
    content_type: String,

    content: AdaptiveCard
}

#[derive(Serialize)]
struct AdaptiveCard {
    version: String,

    #[serde(rename = "type")]
    card_type: String,
    
    body: Vec<Box<CardBody>>
}

#[derive(Serialize)]
struct TextBlock {
    #[serde(rename = "type")]
    body_type: String,

    text: String,
    weight: Option<String>,
    color: Option<String>,
    size: Option<String>,
    spacing: Option<String>,

    #[serde(rename = "horizontalAlignment")]
    horizontal_alignment: Option<String>
}

#[derive(Serialize)]
struct ColumnSet {
    #[serde(rename = "type")]
    body_type: String,

    separator: bool,
    spacing: Option<String>,
    columns: Vec<Column>
}

#[derive(Serialize)]
struct Column {
    #[serde(rename = "type")]
    body_type: String,

    width: String,
    items: Vec<Box<CardBody>>
}

#[derive(Serialize)]
struct ImageBlock {
    #[serde(rename = "type")]
    body_type: String,

    url: String,
    size: Option<String>,
    spacing: Option<String>
}

trait CardBody: erased_serde::Serialize {}
impl CardBody for TextBlock {}
impl CardBody for ColumnSet {}
impl CardBody for ImageBlock {}

serialize_trait_object!(CardBody);

pub struct TeamsToken {
    pub value: String,
    pub expire_date: DateTime<Utc>
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

pub fn handle_message(activity: Activity, bearer_token: &Mutex<TeamsToken>) -> Result<String, Box<Error>> {
    let token: String = get_bearer_token(bearer_token)?;

    println!("{}", token);

    //auajFVRL55[[pylEWN522*!

    let mut core = Core::new().unwrap();
    let client = Client::configure()
        .connector(HttpsConnector::new(4, &core.handle()).unwrap())
        .build(&core.handle());
    let uri = format!("https://webchat.botframework.com/v3/conversations/{}/activities/{}", activity.conversation.id, activity.id).parse().unwrap();
    let mut req = Request::new(Method::Post, uri);

    println!("creating core and request");

    let teams_response = TeamsResponseAdaptive {
        response_type: "message".to_string(),
        from: From { id: activity.recipient.id, name: activity.recipient.name },
        conversation: Conversation { id: activity.conversation.id, name: activity.conversation.name },
        recipient: Recipient { id: activity.from.id, name: activity.from.name },
        attachments: vec!(get_test_adaptive_card()),
        text: None,
        reply_to_id: activity.id
    };

    let json = serde_json::to_string(&teams_response).unwrap();

    println!("creating teams response");

    req.headers_mut().set(header::ContentType::json());
    req.headers_mut().set(header::ContentLength(json.len() as u64));
    req.headers_mut().set(header::Authorization(header::Bearer { token: token }));
    req.set_body(json);

    println!("setting headers");

    let post = client.request(req).and_then(|res| {
        res.body().concat2()
    });

    println!("creating post");

    core.run(post);

    println!("done");

    Ok("Hi there".to_string())
}

fn get_bearer_token(teams_token: &Mutex<TeamsToken>) -> Result<String, Box<Error>> {
    let mut current_token = teams_token.lock().expect("Could not lock mutex");

    if current_token.expire_date >= Utc::now() {
        return Ok(current_token.value.clone())
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

    *current_token = TeamsToken { value: response.access_token.clone(), expire_date: Utc::now() + Duration::seconds(response.expires_in as i64) };

    Ok(response.access_token)
}

fn get_https_client(core: &Core) -> Result<Client<HttpsConnector<client::HttpConnector>>, Box<Error>> {
    let client = ::hyper::Client::configure()
        .connector(::hyper_tls::HttpsConnector::new(4, &core.handle())?)
        .build(&core.handle());
    
    Ok(client)
}

fn get_test_adaptive_card() -> AttachmentAdaptive {
    AttachmentAdaptive {
        content_type: "application/vnd.microsoft.card.adaptive".to_string(),
        content: AdaptiveCard {
            card_type: "AdaptiveCard".to_string(),
            version: "1.0".to_string(),
            body: vec!(
                Box::new(TextBlock { body_type: "TextBlock".to_string(), text: "{action} {crypto}".to_string(), weight: Some("bolder".to_string()), color: None, size: None, spacing: None, horizontal_alignment: None }),
                Box::new(ColumnSet { body_type: "ColumnSet".to_string(), separator: true, spacing: None, columns: vec!(
                    Column { body_type: "Column".to_string(), width: "1".to_string(), items: vec!(
                        Box::new(TextBlock { body_type: "TextBlock".to_string(), text: "Sender".to_string(), weight: None, color: None, size: None, spacing: None, horizontal_alignment: None }),
                        Box::new(TextBlock { body_type: "TextBlock".to_string(), text: "{sender_id}".to_string(), weight: None, color: Some("accent".to_string()), size: Some("large".to_string()), spacing: Some("none".to_string()), horizontal_alignment: None })
                    )},
                    Column { body_type: "Column".to_string(), width: "1".to_string(), items: vec!(
                        Box::new(TextBlock { body_type: "TextBlock".to_string(), text: "".to_string(), weight: None, color: None, size: None, spacing: None, horizontal_alignment: None }),
                        Box::new(ImageBlock { body_type: "Image".to_string(), url: "https://i.imgur.com/6J8tqcM.png".to_string(), size: Some("small".to_string()), spacing: Some("none".to_string()) })
                    )},
                    Column { body_type: "Column".to_string(), width: "1".to_string(), items: vec!(
                        Box::new(TextBlock { body_type: "TextBlock".to_string(), text: "Receiver".to_string(), weight: None, color: None, size: None, spacing: None, horizontal_alignment: Some("right".to_string()) }),
                        Box::new(TextBlock { body_type: "TextBlock".to_string(), text: "{receiver_id}".to_string(), weight: None, color: Some("accent".to_string()), size: Some("large".to_string()), spacing: Some("none".to_string()), horizontal_alignment: Some("right".to_string()) })
                    )}                    
                ) }),
                Box::new(ColumnSet { body_type: "ColumnSet".to_string(), separator: true, spacing: None, columns: vec!(
                    Column { body_type: "Column".to_string(), width: "auto".to_string(), items: vec!(
                        Box::new(ImageBlock { body_type: "Image".to_string(), url: "https://api.qrserver.com/v1/create-qr-code/?data={}".to_string(), size: None, spacing: None })
                    )}
                ) }),
                Box::new(ColumnSet { body_type: "ColumnSet".to_string(), separator: true, spacing: Some("medium".to_string()), columns: vec!(
                    Column { body_type: "Column".to_string(), width: "1".to_string(), items: vec!(
                        Box::new(TextBlock { body_type: "TextBlock".to_string(), text: "Total".to_string(), weight: None, color: None, size: Some("medium".to_string()), spacing: None, horizontal_alignment: None })
                    )},
                    Column { body_type: "Column".to_string(), width: "1".to_string(), items: vec!(                        
                        Box::new(TextBlock { body_type: "TextBlock".to_string(), text: "{amount}".to_string(), weight: Some("bolder".to_string()), color: None, size: Some("medium".to_string()), spacing: None, horizontal_alignment: Some("right".to_string()) })
                    )}
                ) })
            )
        }
    }
}