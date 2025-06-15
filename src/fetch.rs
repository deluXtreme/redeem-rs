use crate::redeem::RedeemableSubscription;
use anyhow::{Context, Result};
use reqwest::Client;
use std::env;

pub async fn fetch_redeemable_subscriptions() -> Result<Vec<RedeemableSubscription>> {
    let api_url =
        env::var("API_URL").unwrap_or_else(|_| "http://localhost:3030/redeemable".to_string());

    let client = Client::new();

    let response = client
        .get(api_url)
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
