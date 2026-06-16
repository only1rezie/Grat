

pub mod context;
pub mod contract_error;
pub mod diagnostic;
pub mod host_error;
pub mod mappings;
pub mod report;

use crate::error::PrismResult;
use crate::types::report::DiagnosticReport;
use crate::xdr::codec::XdrCodec;
use stellar_xdr::curr::{ScVal, TransactionMeta, TransactionResult};

/// Decode `resultMetaXdr` as `TransactionMeta` and, if it is V3, inject the
/// Soroban contract events, diagnostic events, and return value into the JSON
/// payload so downstream enrichment code sees the same shape it does for V1/V2.
///
/// Also extracts `fee_charged` from `resultXdr` so fee details are not lost.
fn parse_v3_metadata(tx_data: &mut serde_json::Value) -> PrismResult<()> {
    // Inject fee_charged from TransactionResult regardless of V3.
    if let Some(result_b64) = tx_data.get("resultXdr").and_then(|r| r.as_str()) {
        if let Ok(tx_result) = TransactionResult::from_xdr_base64(result_b64) {
            tx_data["inclusionFee"] = serde_json::json!(tx_result.fee_charged);
        }
    }

    let meta_b64 = match tx_data.get("resultMetaXdr").and_then(|r| r.as_str()) {
        Some(s) => s.to_string(),
        None => return Ok(()),
    };

    let meta = match TransactionMeta::from_xdr_base64(&meta_b64) {
        Ok(m) => m,
        Err(_) => return Ok(()),
    };

    if let TransactionMeta::V3(v3) = meta {
        let soroban_meta = match v3.soroban_meta {
            Some(s) => s,
            None => return Ok(()),
        };

        // Inject contract events as base64 XDR strings.
        if !soroban_meta.events.is_empty() {
            let contract_events: Vec<String> = soroban_meta
                .events
                .iter()
                .filter_map(|e| XdrCodec::to_xdr_base64(e).ok())
                .collect();
            tx_data["events"] = serde_json::json!({
                "contractEventsXdr": contract_events
            });
        }

        // Inject diagnostic events as base64 XDR strings.
        if !soroban_meta.diagnostic_events.is_empty() {
            let diagnostic_events: Vec<String> = soroban_meta
                .diagnostic_events
                .iter()
                .filter_map(|e| XdrCodec::to_xdr_base64(e).ok())
                .collect();
            tx_data["diagnosticEventsXdr"] = serde_json::json!(diagnostic_events);
        }

        // Encode the return value as a base64 XDR string.
        if soroban_meta.return_value != ScVal::Void {
            if let Ok(b64) = XdrCodec::to_xdr_base64(&soroban_meta.return_value) {
                tx_data["returnValue"] = serde_json::json!(b64);
            }
        }
    }

    Ok(())
}

fn filter_transaction_by_operation(
    tx_data: &mut serde_json::Value,
    op_index: usize,
) -> PrismResult<()> {
    if let Some(events) = tx_data.get_mut("events") {
        if let Some(contract_events) = events.get_mut("contractEventsXdr") {
            if let Some(events_array) = contract_events.as_array_mut() {
                if op_index < events_array.len() {
                    let target_events = events_array[op_index].clone();
                    *events_array = vec![target_events];
                } else {
                    *events_array = vec![];
                }
            }
        }
    }

    if let Some(diagnostic_events) = tx_data.get_mut("diagnosticEventsXdr") {
        if let Some(events_array) = diagnostic_events.as_array_mut() {
            if op_index == 0 && !events_array.is_empty() {
                let first_event = events_array[0].clone();
                *events_array = vec![first_event];
            } else {
                *events_array = vec![];
            }
        }
    }

    Ok(())
}

pub async fn decode_transaction(
    tx_hash: &str,
    network: &crate::types::config::NetworkConfig,
) -> PrismResult<DiagnosticReport> {
    decode_transaction_with_op_filter(tx_hash, network, None).await
}

pub async fn decode_transaction_with_op_filter(
    tx_hash: &str,
    network: &crate::types::config::NetworkConfig,
    op_index: Option<usize>,
) -> PrismResult<DiagnosticReport> {
    let rpc = crate::rpc::SorobanRpcClient::new(network);
    let tx_data = rpc.get_transaction(tx_hash).await?;
    let mut tx_data = serde_json::to_value(tx_data)
        .map_err(|e| crate::error::PrismError::Internal(e.to_string()))?;

    parse_v3_metadata(&mut tx_data)?;

    if let Some(index) = op_index {
        filter_transaction_by_operation(&mut tx_data, index)?;
    }

    let error_info = host_error::classify_error(&tx_data)?;

    let mut report = report::build_report(&error_info)?;

    if error_info.is_contract_error {
        if let Ok(contract_info) = contract_error::resolve(
            &error_info.contract_id.unwrap_or_default(),
            error_info.error_code,
            network,
        )
        .await
        {
            report.contract_error = Some(contract_info);
        }
    }

    diagnostic::enrich_report(&mut report, &tx_data)?;

    context::enrich_report(&mut report, &tx_data)?;

    Ok(report)
}
