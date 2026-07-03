use std::cell::RefCell;
use std::sync::{Mutex, OnceLock, mpsc::Sender};

use crate::filedefs::{TridBuildProgress, TridBuildStage};
use crate::logging::interactive_mode_enabled;

/// Lifecycle phase for weighted operation progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunPhase {
    Analyzing,
    Running,
    Complete,
}

/// A single weighted step in an operation plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressStep {
    pub id: &'static str,
    pub label: &'static str,
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
}

struct OperationPlan {
    phase: RunPhase,
    analyzing_message: String,
    steps: Vec<ProgressStep>,
    active_step: Option<&'static str>,
    committed: bool,
}

impl Default for OperationPlan {
    fn default() -> Self {
        Self {
            phase: RunPhase::Analyzing,
            analyzing_message: String::new(),
            steps: Vec::new(),
            active_step: None,
            committed: false,
        }
    }
}

thread_local! {
    static PLAN: RefCell<OperationPlan> = const { RefCell::new(OperationPlan {
        phase: RunPhase::Analyzing,
        analyzing_message: String::new(),
        steps: Vec::new(),
        active_step: None,
        committed: false,
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
        PLAN.with(|plan| {
            *plan.borrow_mut() = OperationPlan::default();
        });
        Self
    }
}

impl Drop for OperationProgressGuard {
    fn drop(&mut self) {
        PLAN.with(|plan| {
            *plan.borrow_mut() = OperationPlan::default();
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
    }
}

fn compute_overall(plan: &OperationPlan) -> f32 {
    if plan.steps.is_empty() {
        return if plan.phase == RunPhase::Complete {
            1.0
        } else {
            0.0
        };
    }

    let total_weight: u64 = plan.steps.iter().map(|step| step.weight).sum();
    if total_weight == 0 {
        return 0.0;
    }

    let weighted: f64 = plan
        .steps
        .iter()
        .map(|step| {
            let fraction = match step.total.filter(|total| *total > 0) {
                Some(total) => step.current as f64 / total as f64,
                None => 0.0,
            };
            step.weight as f64 * fraction
        })
        .sum();

    (weighted / total_weight as f64).clamp(0.0, 1.0) as f32
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
        Some(total) => format!("{}: {}/{}", step.id, step.current, total),
        None => step.label.to_owned(),
    }
}

fn find_step_index(plan: &mut OperationPlan, id: &'static str) -> Option<usize> {
    plan.steps.iter().position(|step| step.id == id)
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

pub fn begin_single_shot(label: &'static str) {
    with_plan(|plan| {
        plan.phase = RunPhase::Running;
        plan.analyzing_message.clear();
        plan.steps = vec![ProgressStep {
            id: "run",
            label,
            weight: 1,
            current: 0,
            total: None,
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
                step.current = 1;
                step.total = Some(1);
            }
        }
        publish_snapshot(plan);
    });
}

/// Maps TrID build progress into the weighted operation plan.
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
                    plan.steps[index].current = update.current as u64;
                    if !update.message.is_empty() {
                        plan.steps[index].detail = format!("extract: {}", update.message);
                    }
                }
            }
            TridBuildStage::ParseDefinitions => {
                ensure_trid_plan(plan);
                if let Some(total) = update.total.filter(|total| *total > 0) {
                    adjust_step_weight_internal(plan, "parse", total as u64);
                    adjust_step_weight_internal(plan, "reduce", total as u64);
                    set_step_total_internal(plan, "parse", total as u64);
                }
                plan.active_step = Some("parse");
                if let Some(index) = find_step_index(plan, "parse") {
                    plan.steps[index].current = update.current as u64;
                    plan.steps[index].detail = format!(
                        "parse: {}/{}",
                        update.current,
                        update.total.unwrap_or(0)
                    );
                }
            }
            TridBuildStage::ReduceDefinitions => {
                ensure_trid_plan(plan);
                if let Some(total) = update.total.filter(|total| *total > 0) {
                    adjust_step_weight_internal(plan, "reduce", total as u64);
                    set_step_total_internal(plan, "reduce", total as u64);
                }
                plan.active_step = Some("reduce");
                if let Some(index) = find_step_index(plan, "reduce") {
                    plan.steps[index].current = update.current as u64;
                    if let Some(item) = &update.current_item {
                        plan.steps[index].detail =
                            format!("reduce: {}/{} — {}", update.current, update.total.unwrap_or(0), item);
                    } else {
                        plan.steps[index].detail = format!(
                            "reduce: {}/{}",
                            update.current,
                            update.total.unwrap_or(0)
                        );
                    }
                }
            }
            TridBuildStage::FinalizePackage => {
                ensure_trid_plan(plan);
                plan.active_step = Some("finalize");
                if let Some(index) = find_step_index(plan, "finalize") {
                    let total = update.total.unwrap_or(1).max(1) as u64;
                    plan.steps[index].total = Some(total);
                    plan.steps[index].current = update.current as u64;
                    if !update.message.is_empty() {
                        plan.steps[index].detail = format!("finalize: {}", update.message);
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
    if !plan.steps.is_empty() {
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

fn adjust_step_weight_internal(plan: &mut OperationPlan, id: &'static str, weight: u64) {
    if let Some(index) = find_step_index(plan, id) {
        plan.steps[index].weight = weight;
    }
}

fn set_step_total_internal(plan: &mut OperationPlan, id: &'static str, total: u64) {
    if let Some(index) = find_step_index(plan, id) {
        plan.steps[index].total = Some(total);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filedefs::TridBuildStats;

    fn reset_plan() {
        PLAN.with(|plan| {
            *plan.borrow_mut() = OperationPlan::default();
        });
    }

    #[test]
    fn weighted_aggregation_across_steps() {
        reset_plan();
        plan_step("a", "Step A", 1);
        plan_step("b", "Step B", 3);
        commit_plan();
        set_step_total("a", 10);
        set_step_total("b", 100);
        tick_step("a", 10);
        tick_step("b", 50);

        let snapshot = with_plan(|plan| snapshot_from_plan(plan));
        // a done (0.25) + b half (0.375) = 0.625
        assert!((snapshot.overall - 0.625).abs() < 0.01);
        assert_eq!(snapshot.percent, 63);
    }

    #[test]
    fn adjust_step_weight_renormalizes() {
        reset_plan();
        plan_step("parse", "Parse", 100);
        plan_step("reduce", "Reduce", 100);
        commit_plan();
        adjust_step_weight("parse", 200);
        set_step_total("parse", 200);
        tick_step("parse", 100);

        let snapshot = with_plan(|plan| snapshot_from_plan(plan));
        // parse half done: 200 * 0.5 / 300 ≈ 0.333
        assert!((snapshot.overall - 0.333).abs() < 0.02);
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
    fn complete_sets_full_progress() {
        reset_plan();
        begin_single_shot("Running");
        complete_progress();
        let snapshot = with_plan(|plan| snapshot_from_plan(plan));
        assert_eq!(snapshot.phase, RunPhase::Complete);
        assert_eq!(snapshot.percent, 100);
    }
}
