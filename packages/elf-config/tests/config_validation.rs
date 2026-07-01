#![allow(unused_crate_dependencies)]

//! Config validation tests for the ELF configuration loader.

#[path = "config_validation/chunking.rs"] mod chunking;
#[path = "config_validation/context.rs"] mod context;
#[path = "config_validation/core.rs"] mod core;
#[path = "config_validation/helpers.rs"] mod helpers;
#[path = "config_validation/memory_policy.rs"] mod memory_policy;
#[path = "config_validation/ranking.rs"] mod ranking;
#[path = "config_validation/search.rs"] mod search;
#[path = "config_validation/security.rs"] mod security;
