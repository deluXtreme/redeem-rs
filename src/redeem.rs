use alloy::{
    primitives::{Address, U256},
    providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
    sol,
};
use serde::{Deserialize, Serialize};

use alloy::primitives::B256;
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
    pub contract_address: Address,
    pub id: B256,
    pub recipient: Address,
    pub subscriber: Address,
    pub amount: String,
    pub periods: i32,
    pub category: Category,
}

pub async fn redeem_payment(
    signer: PrivateKeySigner,
    subscription: RedeemableSubscription,
) -> Result<bool, Box<dyn std::error::Error>> {
    let provider = ProviderBuilder::new()
        .wallet(signer)
        .connect_http(GNOSIS_RPC.parse()?);
    let contract = SubscriptionModule::new(subscription.contract_address, provider);
    let tx;
    tracing::info!("Redeeming {:#?}", subscription);
    if subscription.category != Category::Trusted {
        tx = contract
            .redeem(subscription.id, vec![].into())
            .send()
            .await?;
    } else {
        let amount = U256::from_str(&subscription.amount)?;
        let periods = U256::from(subscription.periods as u64);
        let params = FindPathParams {
            from: subscription.subscriber,
            to: subscription.recipient,
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
        tx = contract.redeem(subscription.id, data.into()).send().await?;
    }

    tracing::info!(
        "Redeemed {} at: https://gnosisscan.io/tx/{}",
        subscription.id,
        tx.tx_hash()
    );

    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_redeemable_subscription() {
        let json = r#"{
            "contract_address": "0xcebe4b6d50ce877a9689ce4516fe96911e099a78",
            "id": "0x50ede65601819b8885dc3dbf4676204fcd318c26b8281d82af20f69d55b4ca75",
            "subscriber": "0xcf6dc192dc292d5f2789da2db02d6dd4f41f4214",
            "recipient": "0x6b69683c8897e3d18e74b1ba117b49f80423da5d",
            "amount": "10000000000000000",
            "periods": 5,
            "category": "trusted",
            "next_redeem_at": 1775487605
        }"#;

        let sub: RedeemableSubscription = serde_json::from_str(json).unwrap();
        assert_eq!(
            sub.id,
            "0x50ede65601819b8885dc3dbf4676204fcd318c26b8281d82af20f69d55b4ca75"
                .parse::<B256>()
                .unwrap()
        );
        assert_eq!(
            sub.subscriber,
            "0xcf6dc192dc292d5f2789da2db02d6dd4f41f4214"
                .parse::<Address>()
                .unwrap()
        );
        assert_eq!(
            sub.recipient,
            "0x6b69683c8897e3d18e74b1ba117b49f80423da5d"
                .parse::<Address>()
                .unwrap()
        );
        assert_eq!(sub.amount, "10000000000000000");
        assert_eq!(sub.periods, 5);
        assert_eq!(sub.category, Category::Trusted);
    }
}
