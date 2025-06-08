use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
pub struct FindPathParams {
    pub from: String,
    pub to: String,
    pub target_flow: String,
    #[serde(rename = "useWrappedBalances")]
    pub use_wrapped_balances: Option<bool>,
    #[serde(rename = "fromTokens")]
    pub from_tokens: Option<Vec<String>>,
    #[serde(rename = "toTokens")]
    pub to_tokens: Option<Vec<String>>,
    #[serde(rename = "excludeFromTokens")]
    pub exclude_from_tokens: Option<Vec<String>>,
    #[serde(rename = "excludeToTokens")]
    pub exclude_to_tokens: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Transfer {
    pub from: String,
    pub to: String,
    #[serde(rename = "tokenOwner")]
    pub token_owner: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PathfindingResult {
    #[serde(rename = "maxFlow")]
    pub max_flow: String,
    pub transfers: Vec<Transfer>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RpcError {
    code: i32,
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RpcResponse<T> {
    result: Option<T>,
    error: Option<RpcError>,
}

pub async fn find_path(
    rpc_url: &str,
    params: FindPathParams,
) -> Result<PathfindingResult, Box<dyn std::error::Error>> {
    let client = Client::new();

    let request_body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "circlesV2_findPath",
        "params": [{
            "Source": params.from,
            "Sink": params.to,
            "TargetFlow": params.target_flow,
            "WithWrap": params.use_wrapped_balances,
            "FromTokens": params.from_tokens,
            "ToTokens": params.to_tokens,
            "ExcludedFromTokens": params.exclude_from_tokens,
            "ExcludedToTokens": params.exclude_to_tokens,
        }]
    });

    let response = client
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("Pathfinder RPC returned HTTP {}", response.status()).into());
    }

    let json: RpcResponse<PathfindingResult> = response.json().await?;

    match json.result {
        Some(result) => Ok(result),
        None => Err(format!(
            "Pathfinder RPC error: {}",
            serde_json::to_string(&json.error.unwrap_or(RpcError {
                code: -1,
                message: "Unknown error".to_string(),
            }))?
        )
        .into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::U256;
    use std::str::FromStr;
    const CIRCLES_RPC: &str = "https://rpc.aboutcircles.com/"; // Replace with actual RPC URL

    #[tokio::test]
    #[ignore]
    async fn test_find_path() {
        let sender = "0x52e14be00d5acff4424ad625662c6262b4fd1a58";
        let receiver = "0xcf6dc192dc292d5f2789da2db02d6dd4f41f4214";

        // Convert 1 ETH to wei (1e18)
        let value = U256::from_str("1000000000000000000").unwrap().to_string();

        let params = FindPathParams {
            from: sender.to_string(),
            to: receiver.to_string(),
            target_flow: value,
            use_wrapped_balances: Some(true),
            from_tokens: None,
            to_tokens: None,
            exclude_from_tokens: None,
            exclude_to_tokens: None,
        };

        let result = find_path(CIRCLES_RPC, params).await;
        // println!("Path result: {:?}", result);

        // Note: The original test just logs the result, but you might want to add assertions
        // based on your specific requirements
        assert!(result.is_ok(), "find_path should not return an error");
    }
}
