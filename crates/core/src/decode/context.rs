

use crate::error::PrismResult;
use crate::types::report::{DiagnosticReport, FeeBreakdown, ResourceSummary, TransactionContext};

pub fn enrich_report(
    report: &mut DiagnosticReport,
    tx_data: &serde_json::Value,
) -> PrismResult<()> {
    let tx_hash = tx_data
        .get("hash")
        .and_then(|h| h.as_str())
        .unwrap_or("unknown")
        .to_string();

    let ledger_sequence = tx_data.get("ledger").and_then(serde_json::Value::as_u64).unwrap_or(0) as u32;

    let context = TransactionContext {
        tx_hash,
        ledger_sequence,
        function_name: extract_function_name(tx_data),
        arguments: extract_arguments(tx_data),
        return_value: extract_return_value(tx_data),
        fee: extract_fee_breakdown(tx_data),
        resources: extract_resource_summary(tx_data),
    };

    report.transaction_context = Some(context);
    Ok(())
}

fn extract_function_name(tx_data: &serde_json::Value) -> Option<String> {
    tx_data
        .get("functionName")
        .and_then(|f| f.as_str())
        .map(std::string::ToString::to_string)
}

fn extract_arguments(tx_data: &serde_json::Value) -> Vec<String> {
    tx_data
        .get("arguments")
        .and_then(|a| a.as_array())
        .map(|args| args.iter().map(std::string::ToString::to_string).collect())
        .unwrap_or_default()
}

fn extract_return_value(tx_data: &serde_json::Value) -> Option<String> {
    tx_data
        .get("returnValue")
        .and_then(|r| r.as_str())
        .map(std::string::ToString::to_string)
}

fn extract_fee_breakdown(tx_data: &serde_json::Value) -> FeeBreakdown {
    FeeBreakdown {
        inclusion_fee: tx_data
            .get("inclusionFee")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0),
        resource_fee: tx_data
            .get("resourceFee")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0),
        refundable_fee: tx_data
            .get("refundableFee")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0),
        non_refundable_fee: tx_data
            .get("nonRefundableFee")
            .and_then(serde_json::Value::as_i64)
            .unwrap_or(0),
    }
}

fn extract_resource_summary(_tx_data: &serde_json::Value) -> ResourceSummary {
    ResourceSummary {
        cpu_instructions_used: 0,
        cpu_instructions_limit: 0,
        memory_bytes_used: 0,
        memory_bytes_limit: 0,
        read_bytes: 0,
        write_bytes: 0,
    }
}
