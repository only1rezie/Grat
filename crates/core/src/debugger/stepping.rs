//! Step-through execution controller.
//!
//! Provides step-into, step-over, step-out, and continue-to-next-breakpoint
//! operations at host function boundary granularity.

use serde::{Deserialize, Serialize};

/// Step command types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepCommand {
    /// Step into the next host function call or sub-invocation.
    StepInto,
    /// Step over the current host function call.
    StepOver,
    /// Step out of the current contract invocation.
    StepOut,
    /// Continue execution until the next breakpoint.
    Continue,
    /// Run to completion.
    RunToEnd,
}

/// The state visible at each pause point during stepping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PauseState {
    /// The current position in the execution trace.
    pub trace_position: usize,
    /// Current contract being executed.
    pub current_contract: String,
    /// Current function being executed.
    pub current_function: String,
    /// Call stack depth.
    pub call_depth: usize,
    /// Remaining CPU budget.
    pub remaining_cpu: u64,
    /// Remaining memory budget.
    pub remaining_memory: u64,
    /// Current storage values visible to the contract.
    pub visible_storage: Vec<(String, String)>,
    /// Current auth context.
    pub auth_context: Vec<String>,
}

/// Execution stepper — manages step-through execution.
pub struct ExecutionStepper {
    /// Current pause state.
    current_state: Option<PauseState>,
    /// Whether execution is paused.
    is_paused: bool,
}

impl ExecutionStepper {
    /// Create a new execution stepper.
    pub fn new() -> Self {
        Self {
            current_state: None,
            is_paused: false,
        }
    }

    /// Execute a step command.
    pub fn step(&mut self, command: StepCommand) -> Option<&PauseState> {
        tracing::debug!("Stepping: {command:?}");
        self.current_state.as_ref()
    }

    /// Get the current pause state.
    pub fn current_state(&self) -> Option<&PauseState> {
        self.current_state.as_ref()
    }

    /// Check if execution is currently paused.
    pub fn is_paused(&self) -> bool {
        self.is_paused
    }
}

impl Default for ExecutionStepper {
    fn default() -> Self {
        Self::new()
    }
}
