
export interface DiagnosticReport {
  error_category: string;
  error_code: number;
  error_name: string;
  summary: string;
  detailed_explanation: string;
  severity: "info" | "warning" | "error" | "fatal";
  root_causes: RootCause[];
  suggested_fixes: SuggestedFix[];
  contract_error?: ContractErrorInfo;
  transaction_context?: TransactionContext;
}

export interface RootCause {
  description: string;
  likelihood: string;
}

export interface SuggestedFix {
  description: string;
  difficulty: string;
  requires_upgrade: boolean;
  example?: string;
}

export interface ContractErrorInfo {
  contract_id: string;
  error_code: number;
  error_name?: string;
  doc_comment?: string;
}

export interface TransactionContext {
  tx_hash: string;
  ledger_sequence: number;
  function_name?: string;
  arguments: string[];
}

export type Network = "mainnet" | "testnet" | "futurenet" | "custom";
