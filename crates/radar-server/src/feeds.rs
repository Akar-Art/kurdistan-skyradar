//! Upstream live-traffic feeds: FlightRadar24 (zone feed) and ADS-B aggregators.

use std::time::Duration;

use radar_core::ingest::{adsb_feed_urls, fr24_feed_url, parse_adsb_json, parse_fr24_json, RawPosition};
use reqwest::Client;

pub fn http_client() -> Client {
    Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) KurdistanSkyRadar/0.3")
        .timeout(Duration::from_secs(6))
        .build()
        .expect("failed to build HTTP client")
}

/// FlightRadar24 zone feed. Returns `None` if the request failed.
pub async fn fetch_fr24(client: &Client, now: f64) -> Option<Vec<RawPosition>> {
    let url = fr24_feed_url();
    let text = client
        .get(url)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .text()
        .await
        .ok()?;
    parse_fr24_json(&text, now)
}

/// adsb.lol, falling back to adsb.fi. Returns `None` if both failed.
pub async fn fetch_adsb(client: &Client, now: f64) -> Option<Vec<RawPosition>> {
    for url in adsb_feed_urls() {
        let Ok(resp) = client.get(&url).send().await else { continue };
        let Ok(text) = resp.text().await else { continue };
        if let Some(list) = parse_adsb_json(&text, now) {
            return Some(list);
        }
    }
    None
}
