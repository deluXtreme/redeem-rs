mod fetch;
mod redeem;

use alloy::signers::local::PrivateKeySigner;
use std::env;
use tracing_subscriber::FmtSubscriber;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    // Initialize the tracing subscriber
    let subscriber = FmtSubscriber::builder()
        // TODO: Change to DEBUG! https://github.com/deluXtreme/redeem-rs/issues/6
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set tracing subscriber");
    let subscriptions = fetch::fetch_redeemable_subscriptions().await?;
    tracing::info!("Found {} subscriptions", subscriptions.len());
    let signer: PrivateKeySigner = env::var("PK")?.parse()?;
    for subscription in subscriptions {
        redeem::redeem_payment(signer.clone(), subscription).await?;
    }
    Ok(())
}
