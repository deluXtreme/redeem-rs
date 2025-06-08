use std::env;

use alloy::{
    hex,
    primitives::{Address, U256},
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
    sol,
};
use serde::{Deserialize, Serialize};

use crate::{
    circles::find_path,
    flow_matrix::create_flow_matrix,
    redeem::TypeDefinitions::{FlowEdge, Stream},
};
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

    let path = find_path(
        CIRCLES_RPC,
        crate::circles::FindPathParams {
            from: subscription.subscriber.clone(),
            to: subscription.recipient.clone(),
            target_flow: subscription.amount.clone(),
            use_wrapped_balances: Some(true),
            from_tokens: None,
            to_tokens: None,
            exclude_from_tokens: None,
            exclude_to_tokens: None,
        },
    )
    .await?;

    let flow_matrix = create_flow_matrix(
        &subscription.subscriber,
        &subscription.recipient,
        &subscription.amount,
        &path.transfers,
    )?;

    // Convert addresses to alloy::Address
    let flow_vertices: Vec<Address> = flow_matrix
        .flow_vertices
        .iter()
        .map(|addr| Address::from_str(addr).unwrap())
        .collect();

    // Convert FlowEdge to contract FlowEdge
    let flow_edges: Vec<FlowEdge> = flow_matrix
        .flow_edges
        .iter()
        .map(|e| FlowEdge {
            amount: e.amount.parse().unwrap(),
            streamSinkId: e.stream_sink_id,
        })
        .collect();

    // Convert Stream to contract Stream
    let streams: Vec<Stream> = flow_matrix
        .streams
        .iter()
        .map(|s| Stream {
            data: s.data.clone().into(),
            sourceCoordinate: s.source_coordinate,
            flowEdgeIds: s.flow_edge_ids.clone(),
        })
        .collect();

    let module = Address::parse_checksummed(&subscription.module, None)?;
    let sub_id = U256::from_str(&subscription.sub_id)?;
    let packed_coordinates = hex::decode(flow_matrix.packed_coordinates.trim_start_matches("0x"))?;

    let contract = Hub::new(subscription_manager, provider);

    let tx = contract
        .redeemPayment(
            module,
            sub_id,
            flow_vertices,
            flow_edges,
            streams,
            packed_coordinates.into(),
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
