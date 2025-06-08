use std::str::FromStr;

use alloy::{
    hex, primitives::{Address, U256}, providers::ProviderBuilder, sol
};
use serde::{Deserialize, Serialize};

use crate::{
    circles::find_path,
    flow_matrix::{create_flow_matrix, FlowEdge, Stream},
};

sol!( #[allow(missing_docs)] #[sol(rpc)] Hub, "src/redeem.json" );

const CIRCLES_RPC: &str = "https://rpc.aboutcircles.com/";
const SUBSCRIPTION_MANAGER: Address = Address::from_str("0x7E9BaF7CC7cD83bACeFB9B2D5c5124C0F9c30834").expect("Invalid address");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedeemableSubscription {
    pub recipient: String,
    pub subscriber: String,
    pub amount: String,
    pub module: String,
    pub sub_id: u64,
}

pub async fn redeem_payment(
    redeemer: Wallet,
    subscription: RedeemableSubscription,
) -> Result<bool, Box<dyn std::error::Error>> {
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .on_http(CIRCLES_RPC.parse()?);

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
        .map(|addr| Address::parse_checksummed(addr, None).unwrap())
        .collect();

    // Convert FlowEdge to contract FlowEdge
    let flow_edges: Vec<FlowEdge> = flow_matrix
        .flow_edges
        .iter()
        .map(|e| FlowEdge {
            stream_sink_id: e.stream_sink_id,
            amount: e.amount,
        })
        .collect();

    // Convert Stream to contract Stream
    let streams: Vec<Stream> = flow_matrix
        .streams
        .iter()
        .map(|s| Stream {
            source_coordinate: s.source_coordinate,
            flow_edge_ids: s.flow_edge_ids.clone(),
            data: s.data.clone(),
        })
        .collect();

    let module = Address::parse_checksummed(&subscription.module, None)?;
    let sub_id = U256::from(subscription.sub_id);
    let packed_coordinates = hex::decode(flow_matrix.packed_coordinates.trim_start_matches("0x"))?;

    let contract = Hub::new(SUBSCRIPTION_MANAGER, provider);
    
    let tx = contract
        .redeemPayment(
            module,
            sub_id,
            flow_vertices,
            flow_edges,
            streams,
            packed_coordinates,
        )
        .send()
        .await?;

    println!(
        "Redeemed {}-{} at: {}",
        subscription.sub_id, subscription.module, tx.tx_hash()
    );

    let receipt = tx.wait().await?;
    Ok(receipt.status.is_success())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::signers::wallet::WalletBuilder;

    #[tokio::test]
    #[ignore]
    async fn test_redeem_payment() {
        // This test requires a real wallet with funds
        let wallet = WalletBuilder::new()
            .phrase("your test wallet phrase here")
            .build()
            .unwrap();

        let subscription = RedeemableSubscription {
            recipient: "0xcf6dc192dc292d5f2789da2db02d6dd4f41f4214".to_string(),
            subscriber: "0x52e14be00d5acff4424ad625662c6262b4fd1a58".to_string(),
            amount: "1000000000000000000".to_string(), // 1 ETH
            module: "0x7E9BaF7CC7cD83bACeFB9B2D5c5124C0F9c30834".to_string(),
            sub_id: 1,
        };

        let result = redeem_payment(wallet, subscription).await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }
} 