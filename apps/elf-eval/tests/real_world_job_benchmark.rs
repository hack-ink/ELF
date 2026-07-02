#![allow(unused_crate_dependencies)]

//! Integration tests for the real-world job smoke benchmark runner.

#[path = "real_world_job_benchmark/adversarial_quality.rs"] mod adversarial_quality;
#[path = "real_world_job_benchmark/benchmark_core.rs"] mod benchmark_core;
#[path = "real_world_job_benchmark/closeout_reports.rs"] mod closeout_reports;
#[path = "real_world_job_benchmark/competitor_strength.rs"] mod competitor_strength;
#[path = "real_world_job_benchmark/consolidation.rs"] mod consolidation;
#[path = "real_world_job_benchmark/consolidation_knowledge.rs"] mod consolidation_knowledge;
#[path = "real_world_job_benchmark/core_archival.rs"] mod core_archival;
#[path = "real_world_job_benchmark/dreaming_readiness.rs"] mod dreaming_readiness;
#[path = "real_world_job_benchmark/dreaming_reports.rs"] mod dreaming_reports;
#[path = "real_world_job_benchmark/external_adapters.rs"] mod external_adapters;
#[path = "real_world_job_benchmark/live_adapter_tasks.rs"] mod live_adapter_tasks;
#[path = "real_world_job_benchmark/markdown_rendering.rs"] mod markdown_rendering;
#[path = "real_world_job_benchmark/memory_evolution.rs"] mod memory_evolution;
#[path = "real_world_job_benchmark/memory_summary.rs"] mod memory_summary;
#[path = "real_world_job_benchmark/misc_reports.rs"] mod misc_reports;
#[path = "real_world_job_benchmark/operator_debug.rs"] mod operator_debug;
#[path = "real_world_job_benchmark/proactive_brief.rs"] mod proactive_brief;
#[path = "real_world_job_benchmark/production_ops.rs"] mod production_ops;
#[path = "real_world_job_benchmark/quantitative.rs"] mod quantitative;
#[path = "real_world_job_benchmark/recall_debug_reports.rs"] mod recall_debug_reports;
#[path = "real_world_job_benchmark/retrieval.rs"] mod retrieval;
#[path = "real_world_job_benchmark/root_aggregate.rs"] mod root_aggregate;
#[path = "real_world_job_benchmark/scheduled_memory.rs"] mod scheduled_memory;
#[path = "real_world_job_benchmark/support.rs"] mod support;
#[path = "real_world_job_benchmark/trace_replay_reports.rs"] mod trace_replay_reports;
#[path = "real_world_job_benchmark/work_continuity.rs"] mod work_continuity;

use benchmark_core::assert_tracked_external_blocker_row;
