use std::env;

use crate::redeem::TypeDefinitions::{FlowEdge, Stream};
use alloy::{
    primitives::{Address, U256},
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
        target_flow: U256::from_str(&subscription.amount)?,
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
    let contract_matrix = prepare_flow_for_contract(CIRCLES_RPC, params).await?;

    // Convert our generic types to contract-specific types
    let flow_edges: Vec<FlowEdge> = contract_matrix
        .flow_edges
        .into_iter()
        .map(|edge| FlowEdge {
            streamSinkId: edge.stream_sink_id,
            amount: edge.amount.to_string().parse().unwrap(),
        })
        .collect();

    let streams: Vec<Stream> = contract_matrix
        .streams
        .into_iter()
        .map(|stream| Stream {
            sourceCoordinate: stream.source_coordinate,
            flowEdgeIds: stream.flow_edge_ids,
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
            contract_matrix.flow_vertices, // Vec<Address> works directly
            flow_edges,                    // Contract-specific FlowEdge
            streams,                       // Contract-specific Stream
            contract_matrix.packed_coordinates, // Bytes works directly
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
