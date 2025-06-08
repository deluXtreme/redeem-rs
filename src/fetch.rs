use crate::redeem::RedeemableSubscription;
use anyhow::{Context, Result};
use reqwest::Client;

const REDEEMABLE_SUBSCRIPTIONS_URL: &str = "https://subindexer-api.fly.dev/redeemable";

pub async fn fetch_redeemable_subscriptions() -> Result<Vec<RedeemableSubscription>> {
    let client = Client::new();

    let response = client
        .get(REDEEMABLE_SUBSCRIPTIONS_URL)
        .send()
        .await
        .context("Failed to send HTTP request")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("HTTP error! status: {}", response.status()));
    }

    let subscriptions = response
        .json::<Vec<RedeemableSubscription>>()
        .await
        .context("Failed to deserialize JSON")?;

    Ok(subscriptions)
}
