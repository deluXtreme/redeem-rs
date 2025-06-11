use std::env;

use crate::redeem::TypeDefinitions::{FlowEdge, Stream};
use alloy::{
    primitives::{Address, U256, aliases::U192},
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
    sol,
};
use serde::{Deserialize, Serialize};

use circles_pathfinder::{FindPathParams, prepare_flow_for_contract};
use std::str::FromStr;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    Hub,
    "src/redeem.json"
);

const CIRCLES_RPC: &str = "https://rpc.aboutcircles.com/";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedeemableSubscription {
    pub recipient: String,
    pub subscriber: String,
    pub amount: String,
    pub module: String,
    pub sub_id: String,
}

pub async fn redeem_payment(
    subscription: RedeemableSubscription,
) -> Result<bool, Box<dyn std::error::Error>> {
    let subscription_manager = "0x7E9BaF7CC7cD83bACeFB9B2D5c5124C0F9c30834"
        .parse::<Address>()
        .unwrap();

    let signer: PrivateKeySigner = env::var("PK").unwrap().parse().unwrap();
    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http(CIRCLES_RPC.parse()?);

    let params = FindPathParams {
        from: subscription.subscriber.parse::<Address>()?,
        to: subscription.recipient.parse::<Address>()?,
        target_flow: U192::from_str(&subscription.amount)?,
        use_wrapped_balances: Some(true),
        from_tokens: None,
        to_tokens: None,
        exclude_from_tokens: None,
        exclude_to_tokens: None,
    };

    // This automatically:
    // - Finds the optimal path
    // - Creates the flow matrix
    // - Converts to contract-compatible types
    // - Handles flow balancing
    let path_data = prepare_flow_for_contract(CIRCLES_RPC, params).await?;

    // Convert pathfinder types to contract-specific types
    // Types are exactly the same but because they live in different modules
    // Rust treats them as different. Still have to do the conversion :(
    let contract_flow_edges: Vec<FlowEdge> = path_data
        .to_flow_edges()
        .into_iter()
        .map(|edge| FlowEdge {
            streamSinkId: edge.streamSinkId,
            amount: edge.amount,
        })
        .collect();

    let contract_streams = path_data
        .to_streams()
        .into_iter()
        .map(|stream| Stream {
            sourceCoordinate: stream.sourceCoordinate,
            flowEdgeIds: stream.flowEdgeIds,
            data: stream.data,
        })
        .collect();

    let module = Address::parse_checksummed(&subscription.module, None)?;
    let sub_id = U256::from_str(&subscription.sub_id)?;

    let contract = Hub::new(subscription_manager, provider);

    let tx = contract
        .redeemPayment(
            module,
            sub_id,
            path_data.clone().flow_vertices,
            contract_flow_edges,
            contract_streams,
            path_data.to_packed_coordinates(),
        )
        .send()
        .await?;

    println!(
        "Redeemed {}-{} at: {}",
        subscription.sub_id,
        subscription.module,
        tx.tx_hash()
    );

    Ok(true)
}
