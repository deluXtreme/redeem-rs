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
    SubscriptionModule,
    "src/redeem.json"
);

const CIRCLES_RPC: &str = "https://rpc.aboutcircles.com/";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedeemableSubscription {
    pub id: String,
    pub recipient: String,
    pub subscriber: String,
    pub amount: String,
    pub trusted: bool,
}

pub async fn redeem_payment(
    subscription: RedeemableSubscription,
) -> Result<bool, Box<dyn std::error::Error>> {
    let subscription_module = "CHANGE ADDRESS".parse::<Address>().unwrap();

    let signer: PrivateKeySigner = env::var("PK").unwrap().parse().unwrap();
    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http(CIRCLES_RPC.parse()?);
    let contract = SubscriptionModule::new(subscription_module, provider);
    let id = U256::from_str(&subscription.id)?;
    let tx;
    if !subscription.trusted {
        tx = contract.redeemUntrusted(id.into()).send().await?;
    } else {
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

        tx = contract
            .redeem(
                id.into(),
                path_data.clone().flow_vertices,
                contract_flow_edges,
                contract_streams,
                path_data.to_packed_coordinates(),
            )
            .send()
            .await?;
    }

    println!("Redeemed {} at: {}", subscription.id, tx.tx_hash());

    Ok(true)
}
