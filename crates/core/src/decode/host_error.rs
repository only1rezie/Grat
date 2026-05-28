//! Host error classifier.
//!
//! Parses error category + code from TransactionResult XDR and classifies
//! into known error families using the taxonomy database.

use crate::error::{PrismError, PrismResult};
use crate::taxonomy::schema::ErrorCategory;
use crate::xdr::codec::XdrCodec;
use stellar_xdr::curr::{
    InvokeHostFunctionResult, OperationResult, OperationResultTr, ScError, ScErrorCode,
    TransactionResult, TransactionResultResult,
};

/// Classified error information extracted from a transaction result.
#[derive(Debug, Clone)]
pub struct ClassifiedError {
    /// Error category.
    pub category: ErrorCategory,
    /// Numeric error code.
    pub error_code: u32,
    /// Whether this is a contract-defined error (vs host error).
    pub is_contract_error: bool,
    /// Contract ID if this is a contract error.
    pub contract_id: Option<String>,
    /// Raw error data for further processing.
    pub raw_data: serde_json::Value,
}

/// Extract a [`ClassifiedError`] from a decoded [`TransactionResult`] XDR.
///
/// Navigates `TransactionResult → results → OperationResult::OpInner →
/// OperationResultTr::InvokeHostFunction → InvokeHostFunctionResult` and maps
/// the failure variant to the correct error category and code.
///
/// Returns [`PrismError::TransactionSucceeded`] for a successful transaction and
/// [`PrismError::NotSorobanTransaction`] when no `InvokeHostFunction` operation
/// is present.
pub fn from_transaction_result(tx_result: TransactionResult) -> PrismResult<ClassifiedError> {
    let op_results = match tx_result.result {
        TransactionResultResult::TxSuccess(_) => return Err(PrismError::TransactionSucceeded),
        TransactionResultResult::TxFailed(ops) => ops,
        TransactionResultResult::TxFeeBumpInnerSuccess(_) => {
            return Err(PrismError::TransactionSucceeded)
        }
        // Any other top-level failure (TxTooEarly, TxBadSeq, etc.) has no
        // InvokeHostFunction result to inspect.
        _ => return Err(PrismError::NotSorobanTransaction),
    };

    // Find the first InvokeHostFunction operation result.
    let ihf_result = op_results
        .iter()
        .find_map(|op| {
            if let OperationResult::OpInner(OperationResultTr::InvokeHostFunction(r)) = op {
                Some(r.clone())
            } else {
                None
            }
        })
        .ok_or(PrismError::NotSorobanTransaction)?;

    // Map the InvokeHostFunctionResult variant to category + code.
    // The ScError lives in the diagnostic events / meta; here we derive the
    // category from the result code and use 0 as the code for non-contract
    // errors (the taxonomy lookup uses category + code together).
    let (category, error_code, is_contract_error) = match ihf_result {
        InvokeHostFunctionResult::Success(_) => return Err(PrismError::TransactionSucceeded),
        InvokeHostFunctionResult::Trapped => {
            // Trapped means the host function raised an ScError; without the
            // meta we cannot know the exact code, so we default to Contract/0
            // and let the caller enrich from diagnostic events.
            (ErrorCategory::Contract, 0u32, false)
        }
        InvokeHostFunctionResult::ResourceLimitExceeded => (ErrorCategory::Budget, 0, false),
        InvokeHostFunctionResult::EntryArchived => (ErrorCategory::Storage, 0, false),
        InvokeHostFunctionResult::Malformed | InvokeHostFunctionResult::InsufficientRefundableFee => {
            (ErrorCategory::Context, 0, false)
        }
    };

    Ok(ClassifiedError {
        category,
        error_code,
        is_contract_error,
        contract_id: None,
        raw_data: serde_json::Value::Null,
    })
}

/// Classify the error from a transaction result JSON.
///
/// When the response contains a `resultXdr` field the XDR is decoded and
/// [`from_transaction_result`] is used for precise classification.  Otherwise
/// the function falls back to inspecting the JSON status field.
pub fn classify_error(tx_data: &serde_json::Value) -> PrismResult<ClassifiedError> {
    let status = tx_data
        .get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("UNKNOWN");

    if status == "SUCCESS" {
        return Err(PrismError::TransactionSucceeded);
    }

    // Prefer XDR-based classification when the result XDR is available.
    if let Some(result_xdr) = tx_data.get("resultXdr").and_then(|v| v.as_str()) {
        let tx_result = TransactionResult::from_xdr_base64(result_xdr)?;
        let mut classified = from_transaction_result(tx_result)?;
        // Carry the full JSON payload for downstream enrichment.
        classified.raw_data = tx_data.clone();
        return Ok(classified);
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
    use stellar_xdr::curr::{
        Hash, InvokeHostFunctionResult, OperationResult, OperationResultTr, TransactionResult,
        TransactionResultResult, VecM,
    };

    fn make_tx_result(op_result: InvokeHostFunctionResult) -> TransactionResult {
        TransactionResult {
            fee_charged: 100,
            result: TransactionResultResult::TxFailed(
                vec![OperationResult::OpInner(
                    OperationResultTr::InvokeHostFunction(op_result),
                )]
                .try_into()
                .unwrap(),
            ),
            ext: stellar_xdr::curr::TransactionResultExt::V0,
        }
    }

    #[test]
    fn test_parse_error_category() {
        assert_eq!(parse_error_category("budget"), Some(ErrorCategory::Budget));
        assert_eq!(
            parse_error_category("STORAGE"),
            Some(ErrorCategory::Storage)
        );
        assert_eq!(parse_error_category("unknown"), None);
    }

    #[test]
    fn test_from_transaction_result_trapped() {
        let result = make_tx_result(InvokeHostFunctionResult::Trapped);
        let classified = from_transaction_result(result).unwrap();
        assert_eq!(classified.category, ErrorCategory::Contract);
        assert!(!classified.is_contract_error);
    }

    #[test]
    fn test_from_transaction_result_resource_limit() {
        let result = make_tx_result(InvokeHostFunctionResult::ResourceLimitExceeded);
        let classified = from_transaction_result(result).unwrap();
        assert_eq!(classified.category, ErrorCategory::Budget);
    }

    #[test]
    fn test_from_transaction_result_entry_archived() {
        let result = make_tx_result(InvokeHostFunctionResult::EntryArchived);
        let classified = from_transaction_result(result).unwrap();
        assert_eq!(classified.category, ErrorCategory::Storage);
    }

    #[test]
    fn test_from_transaction_result_success_returns_error() {
        let tx_result = TransactionResult {
            fee_charged: 100,
            result: TransactionResultResult::TxSuccess(vec![].try_into().unwrap()),
            ext: stellar_xdr::curr::TransactionResultExt::V0,
        };
        assert!(matches!(
            from_transaction_result(tx_result),
            Err(PrismError::TransactionSucceeded)
        ));
    }

    #[test]
    fn test_from_transaction_result_no_ihf_returns_error() {
        let tx_result = TransactionResult {
            fee_charged: 100,
            result: TransactionResultResult::TxFailed(vec![].try_into().unwrap()),
            ext: stellar_xdr::curr::TransactionResultExt::V0,
        };
        assert!(matches!(
            from_transaction_result(tx_result),
            Err(PrismError::NotSorobanTransaction)
        ));
    }

    #[test]
    fn test_from_transaction_result_ihf_success_returns_error() {
        let result = make_tx_result(InvokeHostFunctionResult::Success(Hash([0; 32])));
        assert!(matches!(
            from_transaction_result(result),
            Err(PrismError::TransactionSucceeded)
        ));
    }
}
