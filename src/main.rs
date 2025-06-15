mod fetch;
mod redeem;

use alloy::signers::local::PrivateKeySigner;
use std::env;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let subscriptions = fetch::fetch_redeemable_subscriptions().await?;
    println!("Found {} subscriptions", subscriptions.len());
    let signer: PrivateKeySigner = env::var("PK")?.parse()?;
    for subscription in subscriptions {
        redeem::redeem_payment(signer.clone(), subscription).await?;
    }
    Ok(())
}
