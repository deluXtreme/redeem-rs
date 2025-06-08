use alloy::{hex, primitives::U256};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TransferStep {
    pub from: String,
    pub to: String,
    #[serde(rename = "tokenOwner")]
    pub token_owner: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FlowEdge {
    pub stream_sink_id: u16,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Stream {
    pub source_coordinate: u16,
    pub flow_edge_ids: Vec<u16>,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FlowMatrix {
    pub flow_vertices: Vec<String>,
    pub flow_edges: Vec<FlowEdge>,
    pub streams: Vec<Stream>,
    pub packed_coordinates: String,
    pub source_coordinate: u16,
}

/// Pack a u16 array into a hex string (big-endian, no padding)
fn pack_coordinates(coords: &[u16]) -> String {
    let mut bytes = Vec::with_capacity(coords.len() * 2);
    for &c in coords {
        bytes.push((c >> 8) as u8);
        bytes.push((c & 0xff) as u8);
    }
    format!("0x{}", hex::encode(bytes))
}

/// Build a sorted vertex list plus index lookup for quick coordinate mapping
fn transform_to_flow_vertices(
    transfers: &[TransferStep],
    from: &str,
    to: &str,
) -> (Vec<String>, HashMap<String, u16>) {
    let mut set = std::collections::HashSet::new();
    set.insert(from.to_lowercase());
    set.insert(to.to_lowercase());

    for t in transfers {
        set.insert(t.from.to_lowercase());
        set.insert(t.to.to_lowercase());
        set.insert(t.token_owner.to_lowercase());
    }

    let mut sorted: Vec<String> = set.into_iter().collect();
    sorted.sort_by(|a, b| {
        let lhs = U256::from_str_radix(a.trim_start_matches("0x"), 16).unwrap();
        let rhs = U256::from_str_radix(b.trim_start_matches("0x"), 16).unwrap();
        lhs.cmp(&rhs)
    });

    let mut idx = HashMap::new();
    for (i, addr) in sorted.iter().enumerate() {
        idx.insert(addr.clone(), i as u16);
    }

    (sorted, idx)
}

/// Create an ABI-ready FlowMatrix object from a list of TransferSteps
pub fn create_flow_matrix(
    from: &str,
    to: &str,
    value: &str,
    transfers: &[TransferStep],
) -> Result<FlowMatrix, Box<dyn std::error::Error>> {
    let sender = from.to_lowercase();
    let receiver = to.to_lowercase();

    let (flow_vertices, idx) = transform_to_flow_vertices(transfers, &sender, &receiver);

    let mut flow_edges: Vec<FlowEdge> = transfers
        .iter()
        .map(|t| {
            let is_terminal = t.to.to_lowercase() == receiver;
            FlowEdge {
                stream_sink_id: if is_terminal { 1 } else { 0 },
                amount: t.value.clone(),
            }
        })
        .collect();

    // Ensure at least one terminal edge
    let has_terminal_edge = flow_edges.iter().any(|e| e.stream_sink_id == 1);
    if !has_terminal_edge {
        let last_edge_index = transfers
            .iter()
            .map(|t| t.to.to_lowercase())
            .rposition(|addr| addr == receiver)
            .unwrap_or(flow_edges.len() - 1);
        flow_edges[last_edge_index].stream_sink_id = 1;
    }

    let term_edge_ids: Vec<u16> = flow_edges
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            if e.stream_sink_id == 1 {
                Some(i as u16)
            } else {
                None
            }
        })
        .collect();

    let streams = vec![Stream {
        source_coordinate: idx[&sender],
        flow_edge_ids: term_edge_ids,
        data: Vec::new(),
    }];

    let mut coords = Vec::new();
    for t in transfers {
        coords.push(idx[&t.token_owner.to_lowercase()]);
        coords.push(idx[&t.from.to_lowercase()]);
        coords.push(idx[&t.to.to_lowercase()]);
    }

    let packed_coordinates = pack_coordinates(&coords);

    let expected = U256::from_str_radix(value, 10)?;
    let terminal_sum: U256 = flow_edges
        .iter()
        .filter(|e| e.stream_sink_id == 1)
        .map(|e| U256::from_str_radix(&e.amount, 10).unwrap())
        .sum();

    if terminal_sum != expected {
        return Err(format!(
            "Terminal sum {} does not equal expected {}",
            terminal_sum, expected
        )
        .into());
    }

    Ok(FlowMatrix {
        flow_vertices,
        flow_edges,
        streams,
        packed_coordinates,
        source_coordinate: idx[&sender],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::U256;

    #[test]
    fn test_pack_coordinates() {
        let coords = vec![0x1234, 0x5678];
        let packed = pack_coordinates(&coords);
        assert_eq!(packed, "0x12345678");
    }

    #[test]
    fn test_transform_to_flow_vertices() {
        let transfers = vec![TransferStep {
            from: "0x1234".to_string(),
            to: "0x5678".to_string(),
            token_owner: "0x9abc".to_string(),
            value: "1000".to_string(),
        }];
        let (sorted, idx) = transform_to_flow_vertices(&transfers, "0x1234", "0x5678");
        assert_eq!(sorted.len(), 3);
        assert_eq!(idx.len(), 3);
    }

    #[test]
    fn test_create_flow_matrix() {
        let sender = "0x52";
        let receiver = "0xcf";
        let value = U256::from_str_radix("1000000000000000000", 10)
            .unwrap()
            .to_string();

        let transfers = vec![
            TransferStep {
                from: sender.to_string(),
                to: "0xa5".to_string(),
                token_owner: sender.to_string(),
                value: value.clone(),
            },
            TransferStep {
                from: "0xa5".to_string(),
                to: "0x63".to_string(),
                token_owner: "0x7b".to_string(),
                value: value.clone(),
            },
            TransferStep {
                from: "0x63".to_string(),
                to: receiver.to_string(),
                token_owner: "0xf7".to_string(),
                value: value.clone(),
            },
        ];

        let result = create_flow_matrix(sender, receiver, &value, &transfers).unwrap();

        assert_eq!(
            result.flow_vertices,
            vec![
                sender.to_string(),
                "0x63".to_string(),
                "0x7b".to_string(),
                "0xa5".to_string(),
                receiver.to_string(),
                "0xf7".to_string(),
            ]
        );

        assert_eq!(
            result.flow_edges,
            vec![
                FlowEdge {
                    stream_sink_id: 0,
                    amount: value.clone()
                },
                FlowEdge {
                    stream_sink_id: 0,
                    amount: value.clone()
                },
                FlowEdge {
                    stream_sink_id: 1,
                    amount: value.clone()
                },
            ]
        );

        assert_eq!(
            result.streams,
            vec![Stream {
                source_coordinate: 0,
                flow_edge_ids: vec![2],
                data: Vec::new(),
            }]
        );

        assert_eq!(
            result.packed_coordinates,
            "0x000000000003000200030001000500010004"
        );
        assert_eq!(result.source_coordinate, 0);
    }

    #[test]
    fn test_create_flow_matrix_terminal_sum_mismatch() {
        let sender = "0x52";
        let receiver = "0xcf";
        let value = U256::from_str_radix("1000000000000000000", 10)
            .unwrap()
            .to_string();
        let bad_value = U256::from_str_radix("100000000000000000", 10)
            .unwrap()
            .to_string(); // 0.1 ETH

        let bad_transfers = vec![TransferStep {
            from: sender.to_string(),
            to: receiver.to_string(),
            token_owner: sender.to_string(),
            value: bad_value,
        }];

        let result = create_flow_matrix(sender, receiver, &value, &bad_transfers);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Terminal sum"));
    }
}
