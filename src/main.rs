mod fetch;
mod redeem;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let subscriptions = fetch::fetch_redeemable_subscriptions().await?;
    println!("Found {} subscriptions", subscriptions.len());
    for subscription in subscriptions {
        redeem::redeem_payment(subscription).await?;
    }
    Ok(())
}
