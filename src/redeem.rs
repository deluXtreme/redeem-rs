use alloy::{
    primitives::{Address, U256},
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
    sol,
};
use serde::{Deserialize, Serialize};

use circles_pathfinder::{FindPathParams, encode_redeem_trusted_data, prepare_flow_for_contract};
use std::str::FromStr;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    contract SubscriptionModule {
        function redeem(bytes32 id, bytes calldata data) external;
    }
);

const GNOSIS_RPC: &str = "https://rpc.gnosischain.com/";
const CIRCLES_RPC: &str = "https://rpc.aboutcircles.com/";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Trusted,
    Untrusted,
    Group,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedeemableSubscription {
    pub id: String,
    pub recipient: String,
    pub subscriber: String,
    pub amount: String,
    pub periods: i32,
    pub category: Category,
}

pub async fn redeem_payment(
    signer: PrivateKeySigner,
    subscription: RedeemableSubscription,
) -> Result<bool, Box<dyn std::error::Error>> {
    let subscription_module = "0xcEbE4B6d50Ce877A9689ce4516Fe96911e099A78".parse()?;

    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http(GNOSIS_RPC.parse()?);
    let contract = SubscriptionModule::new(subscription_module, provider);
    let id = U256::from_str(&subscription.id)?;
    let tx;
    tracing::info!("Redeeming {:#?}", subscription);
    if subscription.category != Category::Trusted {
        tx = contract.redeem(id.into(), vec![].into()).send().await?;
    } else {
        let amount = U256::from_str(&subscription.amount)?;
        let periods = U256::from(subscription.periods as u64);
        let params = FindPathParams {
            from: subscription.subscriber.parse::<Address>()?,
            to: subscription.recipient.parse::<Address>()?,
            target_flow: amount * periods,
            use_wrapped_balances: Some(false),
            from_tokens: None,
            to_tokens: None,
            exclude_from_tokens: None,
            exclude_to_tokens: None,
            simulated_balances: None,
            simulated_trusts: None,
            max_transfers: None,
        };

        // This automatically:
        // - Finds the optimal path
        // - Creates the flow matrix
        // - Converts to contract-compatible types
        // - Handles flow balancing
        let path_data = prepare_flow_for_contract(CIRCLES_RPC, params).await?;
        let data = encode_redeem_trusted_data(
            path_data.flow_vertices,
            path_data.flow_edges,
            path_data.streams,
            path_data.packed_coordinates,
            path_data.source_coordinate,
        );
        tx = contract.redeem(id.into(), data.into()).send().await?;
    }

    tracing::info!(
        "Redeemed {} at: https://gnosisscan.io/tx/{}",
        subscription.id,
        tx.tx_hash()
    );

    Ok(true)
}
