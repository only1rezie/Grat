
pub mod context;
pub mod contract_error;
pub mod diagnostic;
pub mod host_error;
pub mod mappings;
pub mod report;

use crate::error::PrismResult;
use crate::types::report::DiagnosticReport;

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
) -> PrismResult<Vec<DiagnosticReport>> {
    decode_transaction_with_op_filter(tx_hash, network, None).await
}

pub async fn decode_transaction_with_op_filter(
    tx_hash: &str,
    network: &crate::types::config::NetworkConfig,
    op_index: Option<usize>,
) -> PrismResult<Vec<DiagnosticReport>> {
    let rpc = crate::rpc::SorobanRpcClient::new(network);
    let tx_data = rpc.get_transaction(tx_hash).await?;
    let base_tx_data = serde_json::to_value(tx_data)
        .map_err(|e| crate::error::PrismError::Internal(e.to_string()))?;

    // Decode the envelope XDR to determine the number of operations in the transaction.
    let num_ops = if let Some(envelope_str) = base_tx_data.get("envelopeXdr").and_then(|v| v.as_str()) {
        // Use the XDR codec to parse the envelope.
        let envelope = <stellar_xdr::curr::TransactionEnvelope as crate::xdr::codec::XdrCodec>::from_xdr_base64(envelope_str)
            .map_err(|e| crate::error::PrismError::Internal(format!("Failed to decode envelope XDR: {}", e)))?;
        match envelope {
            stellar_xdr::curr::TransactionEnvelope::Tx(v1) => v1.tx.operations.len(),
            stellar_xdr::curr::TransactionEnvelope::TxFeeBump(fb) => {
                // Fee bump transaction contains an inner transaction with its own operations.
                fb.tx.fee_bump_op.operations.len()
            }
        }
    } else {
        // Fallback to a single operation if envelope missing
        1
    };

    let mut reports = Vec::new();
    let indices = match op_index {
        Some(i) => vec![i],
        None => (0..num_ops).collect(),
    };

    for i in indices {
        let mut tx_data = base_tx_data.clone();
        filter_transaction_by_operation(&mut tx_data, i)?;

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
        reports.push(report);
    }

    Ok(reports)
}
