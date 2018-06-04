use std::error::Error;
use hyper::{Uri};
use serde_json;
use tokio_core::reactor::Core;
use futures::{Future, Stream};

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
    #[serde(rename = "USD")]
    usd: CoinmarketcapQuote,

    #[serde(rename = "EUR")]
    eur: CoinmarketcapQuote
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

pub fn get_nano_price_in_euros() -> Result<f32, Box<Error>> {
    let uri: Uri = "https://api.coinmarketcap.com/v2/ticker/1567/?convert=EUR".parse()?;
    let mut core = Core::new()?;
    let client = ::hyper::Client::configure()
        .connector(::hyper_tls::HttpsConnector::new(4, &core.handle())?)
        .build(&core.handle());

    let work = client.get(uri).and_then(|res| {
        res.body().concat2().and_then(move |body| {
            let result: CoinmarketcapInfo = serde_json::from_slice(&body).unwrap();

            return Ok(result)
        })
    });

    Ok(core.run(work)?.data.quotes.eur.price)
}