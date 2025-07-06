use std::env;

use alloy::{
    primitives::{Address, Bytes, U256, aliases::U192},
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
    sol,
};
use serde::{Deserialize, Serialize};

use circles_pathfinder::{FindPathParams, PathData, prepare_flow_for_contract};
use std::str::FromStr;

sol!(
    #[allow(missing_docs)]
    #[sol(rpc)]
    SubscriptionModule,
    "src/redeem.json"
);

sol! {
    struct FlowEdge {
        uint16 streamSinkId;
        uint192 amount;
    }

    struct Stream {
        uint16 sourceCoordinate;
        uint16[] flowEdgeIds;
        bytes data;
    }
}

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
    pub category: Category,
}

pub async fn redeem_payment(
    signer: PrivateKeySigner,
    subscription: RedeemableSubscription,
) -> Result<bool, Box<dyn std::error::Error>> {
    let subscription_module = "0xD5dC464dD561782615D7495d1d7CEd301083c750 ADDRESS".parse::<Address>().unwrap();

    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http(CIRCLES_RPC.parse()?);
    let contract = SubscriptionModule::new(subscription_module, provider);
    let id = U256::from_str(&subscription.id)?;
    let tx;
    if subscription.category != Category::Trusted {
        tx = contract.redeem(id.into(), vec![].into()).send().await?;
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

        tx = contract
            .redeem(id.into(), encode_path_data(path_data))
            .send()
            .await?;
    }

    println!("Redeemed {} at: {}", subscription.id, tx.tx_hash());

    Ok(true)
}

// TODO: Once this works, it can be moved into the circles crate.
fn encode_path_data(data: PathData) -> Bytes {
    let flow_edges: Vec<FlowEdge> = data
        .flow_edges
        .into_iter()
        .map(|(stream_sink_id, amount)| FlowEdge {
            streamSinkId: stream_sink_id,
            amount,
        })
        .collect();

    let streams: Vec<Stream> = data
        .streams
        .into_iter()
        .map(|(source_coord, edge_ids, bytes)| Stream {
            sourceCoordinate: source_coord,
            flowEdgeIds: edge_ids,
            data: bytes.into(),
        })
        .collect();

    // Encode the inner `data` tuple
    let _inner_payload: (Vec<Address>, Vec<FlowEdge>, Vec<Stream>, Vec<u8>, U256) = (
        data.flow_vertices,
        flow_edges,
        streams,
        data.packed_coordinates,
        U256::from(data.source_coordinate),
    );
    // TODO: This shit doesn't work.
    // let encoded_data =
    //     <(Vec<Address>, Vec<FlowEdge>, Vec<Stream>, Vec<u8>, U256)>::abi_encode(&inner_payload);
    // encoded_data.into()
    Bytes::new()
}
