//! Core domain types and wire contracts for `sawhorse`.
//!
//! The definitions moved to `gaugeblock-contracts` (GB-143) along with the
//! `InferenceRequest` builders; this module re-exports them so every historical
//! `sawhorse_core::types::*` path keeps working unchanged.

pub use gaugeblock_contracts::{
    ExecutionMode, GrammarFormat, InferenceRequest, InferenceResponse, ModelRole, ReasoningEffort,
};
