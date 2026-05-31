//! Host error types.
//!
//! `HostError` is the central type the decode engine works with. It covers
//! every Soroban host error category, contract-specific errors, and an
//! `Unknown` variant for forward compatibility.

use serde::Serialize;

use crate::error::{PrismError, PrismResult};
use crate::taxonomy::schema::ErrorCategory;

/// Every Soroban host error category, plus contract-specific and unknown variants.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "category", rename_all = "snake_case")]
pub enum HostError {
    Budget { code: u32 },
    Storage { code: u32 },
    Auth { code: u32 },
    Context { code: u32 },
    Value { code: u32 },
    Object { code: u32 },
    Crypto { code: u32 },
    Contract { code: u32 },
    Wasm { code: u32 },
    Events { code: u32 },
    /// Error defined by a specific deployed contract.
    ContractSpecific {
        contract_id: Option<String>,
        code: u32,
    },
    /// Unrecognised error — preserved for forward compatibility.
    Unknown { type_code: u32, sub_code: u32 },
}

impl HostError {
    /// Human-readable category name.
    pub fn category_name(&self) -> &str {
        match self {
            Self::Budget { .. } => "Budget",
            Self::Storage { .. } => "Storage",
            Self::Auth { .. } => "Auth",
            Self::Context { .. } => "Context",
            Self::Value { .. } => "Value",
            Self::Object { .. } => "Object",
            Self::Crypto { .. } => "Crypto",
            Self::Contract { .. } => "Contract",
            Self::Wasm { .. } => "Wasm",
            Self::Events { .. } => "Events",
            Self::ContractSpecific { .. } => "ContractSpecific",
            Self::Unknown { .. } => "Unknown",
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers kept from the original file
// ---------------------------------------------------------------------------

/// Classified error information extracted from a transaction result.
#[derive(Debug, Clone)]
pub struct ClassifiedError {
    pub category: ErrorCategory,
    pub error_code: u32,
    pub is_contract_error: bool,
    pub contract_id: Option<String>,
    pub raw_data: serde_json::Value,
}

/// Classify the error from a transaction result JSON.
pub fn classify_error(tx_data: &serde_json::Value) -> PrismResult<ClassifiedError> {
    let status = tx_data
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("UNKNOWN");

    if status == "SUCCESS" {
        return Err(PrismError::Internal(
            "Transaction succeeded — no error to classify".to_string(),
        ));
    }

    Ok(ClassifiedError {
        category: ErrorCategory::Contract,
        error_code: 0,
        is_contract_error: false,
        contract_id: None,
        raw_data: tx_data.clone(),
    })
}

/// Map an error category string to an `ErrorCategory` enum value.
pub fn parse_error_category(category_str: &str) -> Option<ErrorCategory> {
    match category_str.to_lowercase().as_str() {
        "budget" => Some(ErrorCategory::Budget),
        "storage" => Some(ErrorCategory::Storage),
        "auth" => Some(ErrorCategory::Auth),
        "context" => Some(ErrorCategory::Context),
        "value" => Some(ErrorCategory::Value),
        "object" => Some(ErrorCategory::Object),
        "crypto" => Some(ErrorCategory::Crypto),
        "contract" => Some(ErrorCategory::Contract),
        "wasm" => Some(ErrorCategory::Wasm),
        "events" => Some(ErrorCategory::Events),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_name() {
        assert_eq!(HostError::Budget { code: 1 }.category_name(), "Budget");
        assert_eq!(HostError::Storage { code: 2 }.category_name(), "Storage");
        assert_eq!(HostError::Auth { code: 3 }.category_name(), "Auth");
        assert_eq!(HostError::Context { code: 0 }.category_name(), "Context");
        assert_eq!(HostError::Value { code: 0 }.category_name(), "Value");
        assert_eq!(HostError::Object { code: 0 }.category_name(), "Object");
        assert_eq!(HostError::Crypto { code: 0 }.category_name(), "Crypto");
        assert_eq!(HostError::Contract { code: 0 }.category_name(), "Contract");
        assert_eq!(HostError::Wasm { code: 0 }.category_name(), "Wasm");
        assert_eq!(HostError::Events { code: 0 }.category_name(), "Events");
        assert_eq!(
            HostError::ContractSpecific { contract_id: None, code: 42 }.category_name(),
            "ContractSpecific"
        );
        assert_eq!(
            HostError::Unknown { type_code: 99, sub_code: 1 }.category_name(),
            "Unknown"
        );
    }

    #[test]
    fn test_serialize_to_json() {
        let err = HostError::ContractSpecific {
            contract_id: Some("CABC123".to_string()),
            code: 3,
        };
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"category\":\"contract_specific\""));
        assert!(json.contains("\"code\":3"));
        assert!(json.contains("CABC123"));
    }

    #[test]
    fn test_unknown_variant() {
        let err = HostError::Unknown { type_code: 7, sub_code: 255 };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["category"], "unknown");
        assert_eq!(json["type_code"], 7);
        assert_eq!(json["sub_code"], 255);
    }

    #[test]
    fn test_parse_error_category() {
        assert_eq!(parse_error_category("budget"), Some(ErrorCategory::Budget));
        assert_eq!(parse_error_category("STORAGE"), Some(ErrorCategory::Storage));
        assert_eq!(parse_error_category("unknown_xyz"), None);
    }
}
