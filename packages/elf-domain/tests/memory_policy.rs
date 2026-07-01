#![allow(unused_crate_dependencies)]

//! Integration tests for memory-policy evaluation.

#[path = "memory_policy/decisions.rs"] mod decisions;
#[path = "memory_policy/precedence.rs"] mod precedence;
#[path = "memory_policy/support.rs"] mod support;
#[path = "memory_policy/thresholds.rs"] mod thresholds;
