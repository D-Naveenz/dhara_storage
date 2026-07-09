//! Interactive operation progress: discover work → commit plan with actual unit totals → tick stages.
//!
//! Design and rollout: [`docs/tui-progress.md`](../../../../docs/tui-progress.md).

use std::cell::RefCell;
use std::sync::{Mutex, OnceLock, mpsc::Sender};
use std::time::Instant;

use crate::filedefs::{TridBuildProgress, TridBuildStage};
use crate::logging::{interactive_mode_enabled, ELAPSED_UI_THRESHOLD};

/// Lifecycle phase for operation progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunPhase {
    Analyzing,
    Running,
    Complete,
}

/// A single step in an operation plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressStep {
    pub id: &'static str,
    pub label: &'static str,
    /// Planned work units; prefer [`set_step_total`] after discovery.
    pub weight: u64,
    pub current: u64,
    pub total: Option<u64>,
    pub detail: String,
}

/// Snapshot published to the interactive UI.
#[derive(Debug, Clone, PartialEq)]
pub struct ProgressSnapshot {
    pub phase: RunPhase,
    pub overall: f32,
    pub percent: u8,
    pub active_step: Option<&'static str>,
    pub step_label: String,
    pub analyzing_message: String,
    /// Fallback activity when no step detail is available (command-level label).
    pub activity_label: String,
    /// Command milestone for panel chrome; does not replace step status.
    pub command_milestone: String,
    /// Wall-clock seconds since command run began; set after [`crate::logging::ELAPSED_UI_THRESHOLD`].
    pub elapsed_secs: Option<u64>,
}

struct OperationPlan {
    phase: RunPhase,
    analyzing_message: String,
    steps: Vec<ProgressStep>,
    active_step: Option<&'static str>,
    committed: bool,
    activity_label: String,
    command_milestone: String,
}

impl Default for OperationPlan {
    fn default() -> Self {
        Self {
            phase: RunPhase::Analyzing,
            analyzing_message: String::new(),
            steps: Vec::new(),
            active_step: None,
            committed: false,
            activity_label: String::new(),
            command_milestone: String::new(),
        }
    }
}

thread_local! {
    static RUN_STARTED: RefCell<Option<Instant>> = const { RefCell::new(None) };
    static PLAN: RefCell<OperationPlan> = const { RefCell::new(OperationPlan {
        phase: RunPhase::Analyzing,
        analyzing_message: String::new(),
        steps: Vec::new(),
        active_step: None,
        committed: false,
        activity_label: String::new(),
        command_milestone: String::new(),
    }) };
}

static INTERACTIVE_PROGRESS_TX: OnceLock<Mutex<Option<Sender<ProgressSnapshot>>>> = OnceLock::new();

pub fn register_interactive_progress_sender(sender: Sender<ProgressSnapshot>) {
    let slot = INTERACTIVE_PROGRESS_TX.get_or_init(|| Mutex::new(None));
    *slot.lock().expect("interactive progress lock poisoned") = Some(sender);
}

pub fn unregister_interactive_progress_sender() {
    if let Some(slot) = INTERACTIVE_PROGRESS_TX.get() {
        *slot.lock().expect("interactive progress lock poisoned") = None;
    }
}

/// Restores the previous plan state when dropped (used by the runner guard).
pub struct OperationProgressGuard;

impl OperationProgressGuard {
    pub fn install() -> Self {
        clear_run_clock();
        PLAN.with(|plan| {
            *plan.borrow_mut() = OperationPlan::default();
        });
        Self
    }
}

impl Drop for OperationProgressGuard {
    fn drop(&mut self) {
        clear_run_clock();
        PLAN.with(|plan| {
            *plan.borrow_mut() = OperationPlan::default();
        });
    }
}

/// Records command start time on the worker thread for elapsed display in snapshots.
pub fn install_run_clock(started: Instant) {
    RUN_STARTED.with(|slot| {
        *slot.borrow_mut() = Some(started);
    });
}

pub fn clear_run_clock() {
    RUN_STARTED.with(|slot| {
        *slot.borrow_mut() = None;
    });
}

fn elapsed_secs_from_clock() -> Option<u64> {
    RUN_STARTED.with(|slot| {
        let Some(started) = *slot.borrow() else {
            return None;
        };
        let elapsed = started.elapsed();
        if elapsed >= ELAPSED_UI_THRESHOLD {
            Some(elapsed.as_secs())
        } else {
            None
        }
    })
}

/// Thin helper for multi-step workflows (quality, package, defs).
#[derive(Debug, Clone, Copy, Default)]
pub struct ProgressSession;

impl ProgressSession {
    pub fn analyzing(&self, message: impl Into<String>) {
        begin_analyzing(message);
    }

    pub fn plan(&self, id: &'static str, label: &'static str, units: u64) {
        plan_step(id, label, units);
    }

    pub fn set_total(&self, id: &'static str, total: u64) {
        set_step_total(id, total);
    }

    pub fn commit(&self) {
        commit_plan();
    }

    pub fn tick(&self, id: &'static str, current: u64, detail: impl Into<String>) {
        tick_step(id, current);
        set_step_message(id, detail);
    }

    pub fn finish_step(&self, id: &'static str, detail: impl Into<String>) {
        with_plan(|plan| {
            if let Some(index) = find_step_index(plan, id) {
                let total = plan.steps[index]
                    .total
                    .unwrap_or(plan.steps[index].weight)
                    .max(1);
                plan.steps[index].current = total;
                plan.steps[index].detail = detail.into();
                plan.active_step = Some(id);
                publish_snapshot(plan);
            }
        });
    }
}

fn with_plan<R>(operation: impl FnOnce(&mut OperationPlan) -> R) -> R {
    PLAN.with(|plan| operation(&mut plan.borrow_mut()))
}

fn publish_snapshot(plan: &OperationPlan) {
    if !interactive_mode_enabled() {
        return;
    }
    let Some(slot) = INTERACTIVE_PROGRESS_TX.get() else {
        return;
    };
    let snapshot = snapshot_from_plan(plan);
    if let Some(sender) = slot.lock().expect("interactive progress lock poisoned").as_ref() {
        let _ = sender.send(snapshot);
    }
}

fn snapshot_from_plan(plan: &OperationPlan) -> ProgressSnapshot {
    let (overall, step_label) = if plan.phase == RunPhase::Analyzing {
        (0.0, plan.analyzing_message.clone())
    } else {
        let overall = compute_overall(plan);
        let label = active_step_label(plan);
        (overall, label)
    };

    let percent = (overall * 100.0).round().clamp(0.0, 100.0) as u8;

    ProgressSnapshot {
        phase: plan.phase,
        overall,
        percent,
        active_step: plan.active_step,
        step_label,
        analyzing_message: plan.analyzing_message.clone(),
        activity_label: plan.activity_label.clone(),
        command_milestone: plan.command_milestone.clone(),
        elapsed_secs: elapsed_secs_from_clock(),
    }
}

/// `percent = sum(min(current, total)) / sum(total) * 100` across all steps.
fn compute_overall(plan: &OperationPlan) -> f32 {
    if plan.steps.is_empty() {
        return if plan.phase == RunPhase::Complete {
            1.0
        } else {
            0.0
        };
    }

    let mut planned_total: u64 = 0;
    let mut done_total: u64 = 0;

    for step in &plan.steps {
        let planned = step.total.unwrap_or(step.weight.max(1));
        let done = step.current.min(planned);
        planned_total += planned;
        done_total += done;
    }

    if planned_total == 0 {
        return 0.0;
    }

    (done_total as f64 / planned_total as f64).clamp(0.0, 1.0) as f32
}

fn active_step_label(plan: &OperationPlan) -> String {
    let Some(active_id) = plan.active_step else {
        return String::new();
    };
    let Some(step) = plan.steps.iter().find(|step| step.id == active_id) else {
        return String::new();
    };

    if !step.detail.is_empty() {
        return step.detail.clone();
    }

    match step.total.filter(|total| *total > 0) {
        Some(total) => format!("{} ({}/{})", step.label, step.current, total),
        None => step.label.to_owned(),
    }
}

fn find_step_index(plan: &mut OperationPlan, id: &'static str) -> Option<usize> {
    plan.steps.iter().position(|step| step.id == id)
}

fn is_placeholder_plan(plan: &OperationPlan) -> bool {
    plan.steps.len() == 1 && plan.steps[0].id == "run"
}

/// Returns true when a handler has committed a multi-step progress plan.
pub fn has_committed_progress_plan() -> bool {
    with_plan(|plan| plan.committed && !plan.steps.is_empty() && !is_placeholder_plan(plan))
}

pub fn begin_analyzing(message: impl Into<String>) {
    with_plan(|plan| {
        plan.phase = RunPhase::Analyzing;
        plan.analyzing_message = message.into();
        plan.steps.clear();
        plan.active_step = None;
        plan.committed = false;
        publish_snapshot(plan);
    });
}

pub fn plan_step(id: &'static str, label: &'static str, weight: u64) {
    with_plan(|plan| {
        if let Some(index) = find_step_index(plan, id) {
            plan.steps[index].weight = weight;
            plan.steps[index].label = label;
        } else {
            plan.steps.push(ProgressStep {
                id,
                label,
                weight,
                current: 0,
                total: None,
                detail: String::new(),
            });
        }
        publish_snapshot(plan);
    });
}

pub fn adjust_step_weight(id: &'static str, weight: u64) {
    with_plan(|plan| {
        if let Some(index) = find_step_index(plan, id) {
            plan.steps[index].weight = weight;
            if plan.steps[index].total.is_none() {
                plan.steps[index].total = Some(weight);
            }
            publish_snapshot(plan);
        }
    });
}

pub fn commit_plan() {
    with_plan(|plan| {
        plan.phase = RunPhase::Running;
        plan.committed = true;
        if let Some(first) = plan.steps.first() {
            plan.active_step = Some(first.id);
        }
        publish_snapshot(plan);
    });
}

/// Legacy single-step placeholder; prefer [`begin_analyzing`] + [`commit_plan`].
pub fn begin_single_shot(label: &'static str) {
    with_plan(|plan| {
        plan.phase = RunPhase::Running;
        plan.analyzing_message.clear();
        plan.steps = vec![ProgressStep {
            id: "run",
            label,
            weight: 1,
            current: 0,
            total: Some(1),
            detail: String::new(),
        }];
        plan.active_step = Some("run");
        plan.committed = true;
        publish_snapshot(plan);
    });
}

pub fn set_step_total(id: &'static str, total: u64) {
    with_plan(|plan| {
        if let Some(index) = find_step_index(plan, id) {
            plan.steps[index].total = Some(total);
            plan.steps[index].weight = total.max(1);
            plan.active_step = Some(id);
            publish_snapshot(plan);
        }
    });
}

pub fn tick_step(id: &'static str, current: u64) {
    with_plan(|plan| {
        if let Some(index) = find_step_index(plan, id) {
            plan.steps[index].current = current;
            plan.active_step = Some(id);
            if !plan.committed && plan.phase == RunPhase::Analyzing {
                plan.phase = RunPhase::Running;
                plan.committed = true;
            }
            publish_snapshot(plan);
        }
    });
}

pub fn set_step_message(id: &'static str, detail: impl Into<String>) {
    with_plan(|plan| {
        if let Some(index) = find_step_index(plan, id) {
            plan.steps[index].detail = detail.into();
            plan.active_step = Some(id);
            publish_snapshot(plan);
        }
    });
}

pub fn complete_progress() {
    with_plan(|plan| {
        plan.phase = RunPhase::Complete;
        for step in &mut plan.steps {
            if let Some(total) = step.total {
                step.current = total;
            } else {
                step.current = step.weight.max(1);
                step.total = Some(step.current);
            }
        }
        publish_snapshot(plan);
    });
}

/// Sets the command milestone shown in action panel chrome (never overwritten by step ticks).
pub fn set_command_milestone(label: &str) {
    with_plan(|plan| {
        plan.command_milestone = label.to_owned();
        publish_snapshot(plan);
    });
}

/// Sets the fallback command-level activity label (used when no step detail is active).
pub fn set_command_activity(label: &str) {
    with_plan(|plan| {
        if has_committed_progress_plan_in(plan) {
            return;
        }
        plan.activity_label = label.to_owned();
        publish_snapshot(plan);
    });
}

pub fn clear_run_activity() {
    with_plan(|plan| {
        plan.activity_label.clear();
        plan.command_milestone.clear();
        publish_snapshot(plan);
    });
    clear_run_clock();
}

fn has_committed_progress_plan_in(plan: &OperationPlan) -> bool {
    plan.committed && !plan.steps.is_empty() && !is_placeholder_plan(plan)
}

/// Maps TrID build progress into the operation plan.
pub fn apply_trid_progress(update: &TridBuildProgress) {
    with_plan(|plan| {
        match update.stage {
            TridBuildStage::LoadSource => {
                if plan.steps.is_empty() && plan.phase == RunPhase::Analyzing {
                    begin_analyzing_internal(plan, "Analyzing TrID source…");
                }
            }
            TridBuildStage::ExtractArchive => {
                ensure_trid_plan(plan);
                plan.active_step = Some("extract");
                if let Some(index) = find_step_index(plan, "extract") {
                    let total = update.total.unwrap_or(1).max(1) as u64;
                    plan.steps[index].total = Some(total);
                    plan.steps[index].weight = total;
                    plan.steps[index].current = update.current as u64;
                    plan.steps[index].detail = if update.message.starts_with("extracted") {
                        "Extracting archive — done".to_owned()
                    } else if total > 1 && update.current > 0 {
                        format!(
                            "Extracting archive ({}/{})",
                            update.current, total
                        )
                    } else if total > 1 {
                        format!("Extracting archive (0/{total})")
                    } else if !update.message.is_empty() {
                        format!("Extracting archive — {}", update.message)
                    } else {
                        "Extracting archive…".to_owned()
                    };
                }
            }
            TridBuildStage::ParseDefinitions => {
                ensure_trid_plan(plan);
                if let Some(total) = update.total.filter(|total| *total > 0) {
                    set_step_total_internal(plan, "parse", total as u64);
                    set_step_total_internal(plan, "reduce", total as u64);
                }
                plan.active_step = Some("parse");
                if let Some(index) = find_step_index(plan, "parse") {
                    plan.steps[index].current = update.current as u64;
                    if update.message.starts_with("Reading definition files") {
                        plan.steps[index].detail = update.message.clone();
                    } else {
                        let total = update.total.unwrap_or(0);
                        plan.steps[index].detail = format!(
                            "Parsing definitions ({}/{})",
                            update.current, total
                        );
                    }
                }
            }
            TridBuildStage::ReduceDefinitions => {
                ensure_trid_plan(plan);
                if let Some(total) = update.total.filter(|total| *total > 0) {
                    set_step_total_internal(plan, "reduce", total as u64);
                }
                plan.active_step = Some("reduce");
                if let Some(index) = find_step_index(plan, "reduce") {
                    plan.steps[index].current = update.current as u64;
                    let total = update.total.unwrap_or(0);
                    if let Some(item) = &update.current_item {
                        plan.steps[index].detail = format!(
                            "Reducing definitions ({}/{}) — {}",
                            update.current, total, item
                        );
                    } else {
                        plan.steps[index].detail =
                            format!("Reducing definitions ({}/{})", update.current, total);
                    }
                }
            }
            TridBuildStage::FinalizePackage => {
                ensure_trid_plan(plan);
                plan.active_step = Some("finalize");
                if let Some(index) = find_step_index(plan, "finalize") {
                    let total = update.total.unwrap_or(1).max(1) as u64;
                    plan.steps[index].total = Some(total);
                    plan.steps[index].weight = total;
                    plan.steps[index].current = update.current as u64;
                    if !update.message.is_empty() {
                        plan.steps[index].detail =
                            format!("Finalizing package — {}", update.message);
                    } else {
                        plan.steps[index].detail = "Finalizing package".to_owned();
                    }
                }
            }
        }

        if plan.committed || !plan.steps.is_empty() {
            if plan.phase == RunPhase::Analyzing && !plan.steps.is_empty() {
                plan.phase = RunPhase::Running;
                plan.committed = true;
            }
            publish_snapshot(plan);
        }
    });
}

fn begin_analyzing_internal(plan: &mut OperationPlan, message: &str) {
    plan.phase = RunPhase::Analyzing;
    plan.analyzing_message = message.to_owned();
}

fn ensure_trid_plan(plan: &mut OperationPlan) {
    if !plan.steps.is_empty() && !is_placeholder_plan(plan) {
        return;
    }
    plan.phase = RunPhase::Running;
    plan.committed = true;
    plan.steps = vec![
        ProgressStep {
            id: "extract",
            label: "Extracting archive",
            weight: 1,
            current: 0,
            total: Some(1),
            detail: String::new(),
        },
        ProgressStep {
            id: "parse",
            label: "Parsing definitions",
            weight: 1,
            current: 0,
            total: None,
            detail: String::new(),
        },
        ProgressStep {
            id: "reduce",
            label: "Reducing definitions",
            weight: 1,
            current: 0,
            total: None,
            detail: String::new(),
        },
        ProgressStep {
            id: "finalize",
            label: "Finalizing package",
            weight: 1,
            current: 0,
            total: Some(1),
            detail: String::new(),
        },
    ];
    plan.active_step = Some("extract");
}

fn set_step_total_internal(plan: &mut OperationPlan, id: &'static str, total: u64) {
    if let Some(index) = find_step_index(plan, id) {
        plan.steps[index].total = Some(total);
        plan.steps[index].weight = total.max(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filedefs::TridBuildStats;
    use std::time::Duration;

    fn reset_plan() {
        PLAN.with(|plan| {
            *plan.borrow_mut() = OperationPlan::default();
        });
    }

    #[test]
    fn unit_sum_aggregation_across_steps() {
        reset_plan();
        plan_step("a", "Step A", 10);
        plan_step("b", "Step B", 100);
        commit_plan();
        set_step_total("a", 10);
        set_step_total("b", 100);
        tick_step("a", 10);
        tick_step("b", 50);

        let snapshot = with_plan(|plan| snapshot_from_plan(plan));
        // 10 + 50 = 60 / 110
        assert!((snapshot.overall - 60.0 / 110.0).abs() < 0.01);
        assert_eq!(snapshot.percent, 55);
    }

    #[test]
    fn placeholder_plan_replaced_by_trid_plan() {
        reset_plan();
        begin_single_shot("Running");
        apply_trid_progress(&TridBuildProgress {
            stage: TridBuildStage::ExtractArchive,
            message: "archive.7z".to_owned(),
            current: 0,
            total: Some(1),
            current_item: None,
            stats: TridBuildStats::default(),
            trace_detail: None,
        });

        let snapshot = with_plan(|plan| snapshot_from_plan(plan));
        assert_eq!(snapshot.active_step, Some("extract"));
        assert!(!snapshot.step_label.is_empty());
        with_plan(|plan| {
            assert!(plan.steps.iter().any(|s| s.id == "parse"));
            assert!(!is_placeholder_plan(plan));
        });
    }

    #[test]
    fn trid_progress_advances_overall() {
        reset_plan();
        apply_trid_progress(&TridBuildProgress {
            stage: TridBuildStage::ParseDefinitions,
            message: String::new(),
            current: 5000,
            total: Some(10000),
            current_item: None,
            stats: TridBuildStats::default(),
            trace_detail: None,
        });

        let snapshot = with_plan(|plan| snapshot_from_plan(plan));
        assert_eq!(snapshot.active_step, Some("parse"));
        assert!(snapshot.overall > 0.0);
        assert!(snapshot.percent > 0);
    }

    #[test]
    fn elapsed_computed_on_publish() {
        reset_plan();
        install_run_clock(Instant::now() - ELAPSED_UI_THRESHOLD - Duration::from_secs(2));
        plan_step("work", "Working", 1);
        commit_plan();
        set_step_message("work", "Running step");

        let snapshot = with_plan(|plan| snapshot_from_plan(plan));
        assert_eq!(snapshot.step_label, "Running step");
        assert!(snapshot.elapsed_secs.unwrap_or(0) >= 6);
    }

    #[test]
    fn elapsed_does_not_clobber_step_detail() {
        reset_plan();
        install_run_clock(Instant::now() - Duration::from_secs(12));
        plan_step("work", "Working", 1);
        commit_plan();
        set_step_message("work", "Running Clippy");

        let snapshot = with_plan(|plan| snapshot_from_plan(plan));
        assert_eq!(snapshot.step_label, "Running Clippy");
        assert_eq!(snapshot.elapsed_secs, Some(12));
        assert!(snapshot.activity_label.is_empty());
    }

    #[test]
    fn complete_sets_full_progress() {
        reset_plan();
        plan_step("only", "Only", 1);
        commit_plan();
        set_step_total("only", 1);
        complete_progress();
        let snapshot = with_plan(|plan| snapshot_from_plan(plan));
        assert_eq!(snapshot.phase, RunPhase::Complete);
        assert_eq!(snapshot.percent, 100);
    }

    #[test]
    fn progress_session_ticks_and_finishes() {
        reset_plan();
        let session = ProgressSession;
        session.analyzing("Planning…");
        session.plan("fmt", "Formatting", 1);
        session.set_total("fmt", 1);
        session.commit();
        session.tick("fmt", 1, "Formatting Rust");
        session.finish_step("fmt", "Formatting Rust — done");

        let snapshot = with_plan(|plan| snapshot_from_plan(plan));
        assert_eq!(snapshot.percent, 100);
    }

    #[test]
    fn thread_local_plan_isolation_prevents_flicker() {
        reset_plan();
        plan_step("parse", "Parsing definitions", 100);
        commit_plan();
        set_step_message("parse", "Parsing definitions (500/10000)");

        let worker = with_plan(|plan| snapshot_from_plan(plan));
        assert_eq!(worker.step_label, "Parsing definitions (500/10000)");

        let foreign = std::thread::spawn(|| with_plan(|plan| snapshot_from_plan(plan)))
            .join()
            .expect("foreign thread join");

        assert!(
            foreign.step_label.is_empty() || foreign.overall == 0.0,
            "foreign thread must not see worker plan state"
        );

        let after = with_plan(|plan| snapshot_from_plan(plan));
        assert_eq!(after.step_label, "Parsing definitions (500/10000)");
    }

    #[test]
    fn extract_progress_advances_step() {
        reset_plan();
        apply_trid_progress(&TridBuildProgress {
            stage: TridBuildStage::ExtractArchive,
            message: "Extracting archive (50/200)".to_owned(),
            current: 50,
            total: Some(200),
            current_item: None,
            stats: TridBuildStats::default(),
            trace_detail: None,
        });

        let snapshot = with_plan(|plan| snapshot_from_plan(plan));
        assert_eq!(snapshot.active_step, Some("extract"));
        assert!(snapshot.step_label.contains("50/200"));
        with_plan(|plan| {
            let extract = plan
                .steps
                .iter()
                .find(|step| step.id == "extract")
                .expect("extract step");
            assert_eq!(extract.current, 50);
            assert_eq!(extract.total, Some(200));
        });
    }

    #[test]
    fn enumerate_progress_updates_parse_detail() {
        reset_plan();
        apply_trid_progress(&TridBuildProgress {
            stage: TridBuildStage::ParseDefinitions,
            message: "Reading definition files (21692 found)".to_owned(),
            current: 0,
            total: Some(21692),
            current_item: None,
            stats: TridBuildStats::default(),
            trace_detail: None,
        });

        let snapshot = with_plan(|plan| snapshot_from_plan(plan));
        assert_eq!(snapshot.active_step, Some("parse"));
        assert_eq!(
            snapshot.step_label,
            "Reading definition files (21692 found)"
        );
    }
}
