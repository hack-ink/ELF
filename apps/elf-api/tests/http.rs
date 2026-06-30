#![allow(unused_crate_dependencies)]

//! End-to-end HTTP integration tests for the ELF API app.

#[path = "http/helpers.rs"] pub(crate) mod helpers;

#[path = "http/auth_admin.rs"] mod auth_admin;
#[path = "http/contract.rs"] mod contract;
#[path = "http/request_validation.rs"] mod request_validation;
#[path = "http/sharing.rs"] mod sharing;
