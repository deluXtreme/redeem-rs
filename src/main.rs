mod fetch;
mod redeem;

use alloy::signers::local::PrivateKeySigner;
use reqwest::Url;
use std::env;
use tracing_subscriber::FmtSubscriber;

struct Config {
    signer: PrivateKeySigner,
    api_url: Url,
}

impl Config {
    fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            signer: env::var("PK")?.parse()?,
            api_url: env::var("API_URL")
                .unwrap_or_else(|_| "http://localhost:3030/redeemable".to_string())
                .parse()?,
        })
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let subscriber = FmtSubscriber::builder()
        // TODO: Change to DEBUG! https://github.com/deluXtreme/redeem-rs/issues/6
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");

    let config = Config::from_env()?;
    let subscriptions = fetch::fetch_redeemable_subscriptions(config.api_url).await?;
    tracing::info!("Found {} subscriptions", subscriptions.len());
    for subscription in subscriptions {
        tracing::info!("Redeeming {:#?}", subscription);
        let tx_hash = redeem::redeem_payment(config.signer.clone(), subscription).await?;
        tracing::info!("Redeemed at: https://gnosisscan.io/tx/{}", tx_hash);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn test_redeem_one() {
        dotenv::dotenv().ok();
        let config = Config::from_env().expect("Failed to load config");
        let subscriptions = fetch::fetch_redeemable_subscriptions(config.api_url)
            .await
            .expect("Failed to fetch redeemable subscriptions");
        if let Some(subscription) = subscriptions.first().cloned() {
            let result = redeem::redeem_payment(config.signer, subscription).await;
            assert!(result.is_ok(), "redeem_payment failed: {:?}", result.err());
        }
    }
}
